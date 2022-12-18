use std::{net::{TcpListener, SocketAddr}, thread, sync::mpsc::{Receiver, Sender, channel}, collections::HashSet};

use hmac::{Hmac, Mac};
use log::{info, warn, debug, trace};
use serde_json::json;
use sha2::Sha256;

use crate::{protocol::{data::PacketEncoder, serverbound::*, clientbound::*, command::Commands, Position}, config::{Config, LoginMode}};
use crate::plugins::{Plugins, Response};
use crate::VERSION;

use super::{client::NetworkClient, Player};

pub struct NetworkServer<'lua> {
    plugins: Plugins<'lua>,
    commands: Commands,
    new_clients: Receiver<NetworkClient>,
    clients: Vec<NetworkClient>,
    config: Config,
}

impl <'lua> NetworkServer<'lua> {
    pub fn new(config: Config, mut plugins: Plugins<'lua>) -> Self {
        let (send, recv) = channel();
        info!("Initializing plugins");
        plugins.init();
        let mut commands = Commands::new();
        commands.create_simple_cmd("qc");
        let commands = plugins.register_commands(commands).unwrap();
        thread::spawn(move || Self::listen(&SocketAddr::new(config.addr, config.port), send));
        Self {
            config,
            plugins,
            commands,
            new_clients: recv,
            clients: Vec::new(),
        }
    }

    fn listen(addr: &SocketAddr, send_clients: Sender<NetworkClient>) {
        info!("Listening on {}", addr);
        let listener = TcpListener::bind(addr).unwrap();
        for (id, stream) in listener.incoming().enumerate() {
            let stream = stream.unwrap();
            debug!("Connection from {} (id {})", stream.peer_addr().unwrap(), id);
            let stream_2 = stream.try_clone().unwrap();
            let (send, recv) = channel();
            thread::spawn(|| NetworkClient::listen(stream_2, send));
            let client = NetworkClient {
                id: id as i32,
                verified: false,
                closed: false,
                stream,
                serverbound: recv,
                player: None,
            };
            send_clients.send(client).unwrap();
        }
    }

    pub fn get_new_clients(&mut self) {
        while let Ok(client) = self.new_clients.try_recv() {
            self.clients.push(client);
        }
    }

    pub fn send_keep_alive(&mut self) {
        let mut closed = Vec::new();
        for client in self.clients.iter_mut() {
            if client.player.is_some() {
                let result = client.send_packet(KeepAlive { data: 0 });
                if result.is_err() {
                    client.close();
                    self.plugins.player_leave(client.player.as_ref().unwrap());
                    closed.push(client.id);
                }
            }
        }
        self.clients.retain(|x| !closed.contains(&x.id));
    }

    pub fn handle_connections(&mut self) {
        let mut closed = HashSet::new();
        for i in 0..self.clients.len() {
            let client: &mut NetworkClient = unsafe {
                &mut *(self.clients.get_unchecked_mut(i) as *mut _)
            };
            let mut alive = true;
            while let Some(packet) = client.recv_packet(&mut alive) {
                let result = self.handle_packet(client, packet);
                if result.is_err() {
                    warn!("error: {}", result.unwrap_err());
                    alive = false;
                    break
                }
            }
            if !alive && !client.closed {
                closed.insert(client.id);
                if let Some(pl) = &client.player {
                    self.plugins.player_leave(pl);
                }
                client.close();
            }
        }
        for response in self.plugins.get_responses() {
            let _ = self.handle_plugin_response(response);
        }
        self.clients.retain(|x| !closed.contains(&x.id) && !x.closed);
    }

    fn handle_plugin_response(&mut self, response: Response) -> std::io::Result<()> {
        match response {
            Response::Message { player, message } => {
                for client in self.clients.iter_mut() {
                    if let Some(p) = &client.player {
                        if p.name == player || p.uuid.to_string() == player {
                            client.send_packet(SystemChatMessage { message, overlay: false })?;
                            break
                        }
                    }
                }
            },
            Response::Broadcast { message } => {
                for client in self.clients.iter_mut() {
                    if client.player.is_some() {
                        client.send_packet(SystemChatMessage { message: message.clone(), overlay: false })?;
                    }
                }
            },
            Response::Disconnect { player, reason } => {
                for client in self.clients.iter_mut() {
                    if let Some(pl) = &client.player {
                        if pl.name == player || pl.uuid.to_string() == player {
                            client.send_packet(Disconnect { reason: reason.clone() })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_packet(&mut self, client: &mut NetworkClient, packet: ServerBoundPacket) -> Result<(), Box<dyn std::error::Error>> {
        trace!("Recieved packet from client {}:", client.id);
        match packet {
            ServerBoundPacket::Ignored(_) => (),
            ServerBoundPacket::Unknown(id) => warn!("Unknown packet: {}", id),
            ServerBoundPacket::Handshake(_) => (),
            ServerBoundPacket::StatusRequest() 
                => client.send_packet(StatusResponse {
                    data: r#"{"version":{"name":"1.19.3","protocol":761}}"#.to_owned()
                })?,
            ServerBoundPacket::PingRequest(n) => {
                client.send_packet(PingResponse { data: n })?;
                client.close();
            }
            ServerBoundPacket::LoginStart(login_start) 
                => self.start_login(client, login_start)?,
            ServerBoundPacket::LoginPluginResponse(LoginPluginResponse { id: 10, data }) 
                => self.velocity_login(client, data)?,
            ServerBoundPacket::LoginPluginResponse(LoginPluginResponse{ id: -1, .. }) 
                => self.login(client)?,
            ServerBoundPacket::LoginPluginResponse { .. } => {
                client.send_packet(LoginDisconnect { reason: json!({"text": "bad plugin response"}) })?;
                client.close();
            }
            ServerBoundPacket::ChatMessage(msg) => {
                self.plugins.chat_message(client.player.as_ref().unwrap(), &msg.message);
            }
            ServerBoundPacket::ChatCommand(msg) => {
                let mut parts = msg.message.splitn(2, ' ');
                if let Some(cmd) = parts.next() {
                    if cmd == "qc" {
                        client.send_packet(SystemChatMessage { message: json!({
                            "text": format!("QuectoCraft version {}", VERSION),
                            "color": "green"
                        }), overlay: false })?;
                    } else {
                        let args = parts.next().unwrap_or_default();
                        self.plugins.command(client.player.as_ref().unwrap(), cmd, args);
                    }
                }
            }
        }
        Ok(())
    }

    fn start_login(&mut self, client: &mut NetworkClient, login_start: LoginStart) -> Result<(), Box<dyn std::error::Error>> {
        if self.clients.iter().filter_map(|x| x.player.as_ref()).any(|x| x.uuid == login_start.uuid) {
            client.send_packet(LoginDisconnect { reason: json!({
                "translate": "multiplayer.disconnect.duplicate_login"
            })})?;
            client.close();
            return Ok(())
        }
        client.player = Some(Player {
            name: login_start.name.clone(),
            uuid: login_start.uuid,
        });
        match self.config.login {
            LoginMode::Offline => {
                client.verified = true;
                client.send_packet(LoginPluginRequest{ 
                    id: -1, 
                    channel: "qc:init".to_owned(), 
                    data: Vec::new() 
                })?;
            },
            LoginMode::Velocity => {
                client.send_packet(LoginPluginRequest{ 
                    id: 10, 
                    channel: "velocity:player_info".to_owned(), 
                    data: vec![1],
                })?
            }
        }
        Ok(())
    }

    fn velocity_login(&mut self, client: &mut NetworkClient, data: Option<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(data) = data else {
            client.send_packet(LoginDisconnect { reason: json!({
                "text": "This server can only be connected to via a Velocity proxy",
                "color": "red"
            })})?;
            client.close();
            return Ok(());
        };
        let (sig, data) = data.split_at(32);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.velocity_secret.clone().unwrap().as_bytes())?;
        mac.update(data);
        if mac.verify_slice(sig).is_err() {
            client.send_packet(Disconnect { reason: json!({ 
                "text": "Could not verify secret. Ensure that the secrets configured for Velocity and Quectocraft match."
            })})?;
            client.close();
            return Ok(())
        }
        client.verified = true;
        client.send_packet(LoginPluginRequest{ 
            id: -1, 
            channel: "qc:init".to_owned(), 
            data: Vec::new() 
        })?;
        Ok(())
    }

    fn login(&mut self, client: &mut NetworkClient) -> std::io::Result<()> {
        if !client.verified {
            client.send_packet(Disconnect { reason: json!({
                "text": "Failed to verify your connection",
                "color": "red",
            })})?;
            client.close();
            return Ok(())
        }
        client.send_packet(LoginSuccess {
            name: client.player.as_ref().unwrap().name.to_owned(),
            uuid: client.player.as_ref().unwrap().uuid,
        })?;
        self.plugins.player_join(client.player.as_ref().unwrap());
        client.send_packet(LoginPlay {
            eid: client.id,
            is_hardcore: false,
            gamemode: 3,
            prev_gamemode: 3,
            dimensions: vec![
                "qc:world".to_owned(), 
            ],
            registry_codec: include_bytes!("../resources/registry_codec.nbt").to_vec(),
            dimension_type: "minecraft:the_end".to_owned(),
            dimension_name: "qc:world".to_owned(),
            seed_hash: 0,
            max_players: 0,
            view_distance: 8,
            sim_distance: 8,
            reduced_debug_info: false,
            respawn_screen: false,
            is_debug: false,
            is_flat: false,
            death_location: None,
        })?;
        client.send_packet(PluginMessage {
            channel: "minecraft:brand".to_owned(),
            data: {
                let mut data = Vec::new();
                data.write_string(32767, &format!("Quectocraft {}", VERSION));
                data
            }
        })?;
        client.send_packet(self.commands.clone())?;
        let mut chunk_data: Vec<u8> = Vec::new();
        for _ in 0..(384 / 16) {
            // number of non-air blocks
            chunk_data.write_short(0);
            // block states
            chunk_data.write_ubyte(0);
            chunk_data.write_varint(0);
            chunk_data.write_varint(0);
            // biomes
            chunk_data.write_ubyte(0);
            chunk_data.write_varint(0);
            chunk_data.write_varint(0);
        }
        let hmdata = vec![0i64; 37];
        let mut heightmap = nbt::Blob::new();
        heightmap.insert("MOTION_BLOCKING", hmdata).unwrap();
        client.send_packet(ChunkData {
            x: 0,
            z: 0,
            heightmap,
            chunk_data,
        })?;
        client.send_packet(SetDefaultSpawnPosition {
            pos: Position { x: 0, y: 0, z: 0 }, angle: 0.0
        })?;
        client.send_packet(SyncPlayerPosition {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            flags: 0,
            teleport_id: 0,
            dismount: false
        })?;
        // TODO why doesn't this work with quilt?
        // client.send_packet(ClientBoundPacket::PlayerAbilities(0x0f, 0.05, 0.1))?;
        Ok(())
    }

}
