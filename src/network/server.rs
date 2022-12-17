use std::{net::{TcpListener, SocketAddr}, thread, sync::mpsc::{Receiver, Sender, channel}, collections::HashSet};

use log::{info, warn, debug};
use serde_json::json;

use crate::protocol::{data::PacketEncoder, serverbound::*, clientbound::*, command::Commands, Position};
use crate::plugins::{Plugins, Response};
use crate::VERSION;

use super::{client::NetworkClient, Player};

pub struct NetworkServer<'lua> {
    plugins: Plugins<'lua>,
    commands: Commands,
    new_clients: Receiver<NetworkClient>,
    clients: Vec<NetworkClient>,
}

impl <'lua> NetworkServer<'lua> {
    pub fn new(addr: SocketAddr, mut plugins: Plugins<'lua>) -> Self {
        let (send, recv) = channel();
        info!("Initializing plugins");
        plugins.init();
        let mut commands = Commands::new();
        commands.create_simple_cmd("qc");
        let commands = plugins.register_commands(commands).unwrap();
        thread::spawn(move || Self::listen(&addr, send));
        Self { 
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
                play: false,
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
            if client.play {
                let result = client.send_packet(ClientBoundPacket::KeepAlive(0));
                if result.is_err() {
                    client.close();
                    if let Some(pl) = &client.player {
                        self.plugins.player_leave(pl);
                    }
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
        self.clients.retain(|x| !closed.contains(&x.id));
    }

    fn handle_plugin_response(&mut self, response: Response) -> std::io::Result<()> {
        match response {
            Response::Message { player, message } => {
                for client in self.clients.iter_mut() {
                    if let Some(p) = &client.player {
                        if p.name == player || p.uuid.to_string() == player {
                            client.send_packet(ClientBoundPacket::SystemChatMessage(message, false))?;
                            break
                        }
                    }
                }
            },
            Response::Broadcast { message } => {
                for client in self.clients.iter_mut() {
                    if client.player.is_some() {
                        client.send_packet(ClientBoundPacket::SystemChatMessage(message.clone(), false))?;
                    }
                }
            },
            Response::Disconnect { player, reason } => {
                for client in self.clients.iter_mut() {
                    if let Some(pl) = &client.player {
                        if pl.name == player || pl.uuid.to_string() == player {
                            client.send_packet(ClientBoundPacket::Disconnect(reason.clone()))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_packet(&mut self, client: &mut NetworkClient, packet: ServerBoundPacket) -> std::io::Result<()> {
        match packet {
            ServerBoundPacket::Ignored(_) => (),
            ServerBoundPacket::Unknown(id) => warn!("Unknown packet: {}", id),
            ServerBoundPacket::Handshake(_) => (),
            ServerBoundPacket::StatusRequest() 
                => client.send_packet(ClientBoundPacket::StatusResponse(
                    r#"{"version":{"name":"1.19.3","protocol":761}}"#.to_owned()
                ))?,
            ServerBoundPacket::PingRequest(n) => {
                client.send_packet(ClientBoundPacket::PingResponse(n))?;
                client.close();
            }
            ServerBoundPacket::LoginStart(login_start) => {
                if self.clients.iter().filter_map(|x| x.player.as_ref()).any(|x| x.uuid == login_start.uuid) {
                    client.send_packet(ClientBoundPacket::LoginDisconnect(json!({"translate": "multiplayer.disconnect.duplicate_login"})))?;
                    client.close();
                } else {
                    client.player = Some(Player {
                        name: login_start.name.clone(),
                        uuid: login_start.uuid,
                    });
                    client.play = true;
                    client.send_packet(ClientBoundPacket::LoginSuccess(LoginSuccess {
                        name: login_start.name,
                        uuid: login_start.uuid,
                    }))?;
                    self.plugins.player_join(client.player.as_ref().unwrap());
                    self.post_login(client)?;
                }
            }
            ServerBoundPacket::ChatMessage(msg) => {
                self.plugins.chat_message(client.player.as_ref().unwrap(), &msg.message);
            }
            ServerBoundPacket::ChatCommand(msg) => {
                let mut parts = msg.message.splitn(1, " ");
                if let Some(cmd) = parts.next() {
                    if cmd == "qc" {
                        client.send_packet(ClientBoundPacket::SystemChatMessage(json!({
                            "text": format!("QuectoCraft version {}", VERSION),
                            "color": "green"
                        }), false))?;
                    } else {
                        let args = parts.next().unwrap_or_default();
                        self.plugins.command(client.player.as_ref().unwrap(), cmd, args);
                    }
                }
            }
        }
        Ok(())
    }

    fn post_login(&mut self, client: &mut NetworkClient) -> std::io::Result<()> {
        client.send_packet(ClientBoundPacket::LoginPlay(LoginPlay {
            eid: client.id,
            is_hardcore: false,
            gamemode: 1,
            prev_gamemode: 1,
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
        }))?;
        client.send_packet(ClientBoundPacket::PluginMessage(PluginMessage {
            channel: "minecraft:brand".to_owned(),
            data: {
                let mut data = Vec::new();
                data.write_string(32767, &format!("Quectocraft {}", VERSION));
                data
            }
        }))?;
        client.send_packet(ClientBoundPacket::Commands(self.commands.clone()))?;
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
        client.send_packet(ClientBoundPacket::ChunkData(ChunkData {
            x: 0,
            z: 0,
            heightmap,
            chunk_data,
        }))?;
        client.send_packet(ClientBoundPacket::SetDefaultSpawnPosition(
            Position { x: 0, y: 0, z: 0 }, 0.0
        ))?;
        client.send_packet(ClientBoundPacket::SyncPlayerPosition(SyncPlayerPosition {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            flags: 0,
            teleport_id: 0,
            dismount: false
        }))?;
        // TODO why doesn't this work with quilt?
        // client.send_packet(ClientBoundPacket::PlayerAbilities(0x0f, 0.05, 0.1))?;
        Ok(())
    }

}
