use std::{net::TcpListener, thread, sync::mpsc::{Receiver, Sender, channel}, collections::HashSet};

use log::{info, warn, trace, debug, error};

use crate::{protocol::{data::PacketEncoder, serverbound::*, clientbound::*}, plugins::{Plugins, Response}, VERSION};

use super::{client::NetworkClient, Player};

pub struct NetworkServer<'lua> {
    plugins: Plugins<'lua>,
    new_clients: Receiver<NetworkClient>,
    clients: Vec<NetworkClient>,
}

impl <'lua> NetworkServer<'lua> {
    pub fn new(addr: String, plugins: Plugins<'lua>) -> Self {
        let (send, recv) = channel();
        info!("initializing plugins");
        plugins.init();
        thread::spawn(move || Self::listen(&addr, send));
        Self { 
            plugins,
            new_clients: recv,
            clients: Vec::new(),
        }
    }

    fn listen(addr: &str, send_clients: Sender<NetworkClient>) {
        info!("listening on {}", addr);
        let listener = TcpListener::bind(addr).unwrap();
        for (id, stream) in listener.incoming().enumerate() {
            let stream = stream.unwrap();
            info!("connection from {} (id {})", stream.peer_addr().unwrap(), id);
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
                if let Err(_) = client.send_packet(ClientBoundPacket::KeepAlive(0)) {
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
                if let Err(_) = self.handle_packet(client, packet) {
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
            Response::Log { level, origin, message } => {
                match level {
                    0 => trace!(target: &origin, "{}", message),
                    1 => debug!(target: &origin, "{}", message),
                    2 => info!(target: &origin, "{}", message),
                    3 => warn!(target: &origin, "{}", message),
                    4 => error!(target: &origin, "{}", message),
                    _ => warn!("unknown log level: {}", level)
                }
            },
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
        }
        Ok(())
    }

    fn handle_packet(&mut self, client: &mut NetworkClient, packet: ServerBoundPacket) -> std::io::Result<()> {
        match packet {
            ServerBoundPacket::Ignored(_) => (),
            ServerBoundPacket::Unknown(id) => warn!("unknown: {}", id),
            ServerBoundPacket::Handshake(_) => (),
            ServerBoundPacket::StatusRequest() 
                => client.send_packet(ClientBoundPacket::StatusResponse(
                    r#"{"version":{"name":"1.19.2","protocol":760}}"#.to_owned()
                ))?,
            ServerBoundPacket::PingRequest(n) => {
                client.send_packet(ClientBoundPacket::PingResponse(n))?;
                client.close();
            }
            ServerBoundPacket::LoginStart(login_start) => {
                client.player = Some(Player {
                    name: login_start.name.clone(),
                    uuid: login_start.uuid.unwrap(),
                });
                client.play = true;
                client.send_packet(ClientBoundPacket::LoginSuccess(LoginSuccess {
                    name: login_start.name,
                    uuid: login_start.uuid.unwrap(),
                }))?;
                self.plugins.player_join(client.player.as_ref().unwrap());
                self.post_login(client)?;
            }
            ServerBoundPacket::ChatMessage(msg) => {
                self.plugins.chat_message(client.player.as_ref().unwrap(), &msg.message);
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
                "minecraft:world".to_owned(), 
            ],
            registry_codec: include_bytes!("../resources/registry_codec.nbt").to_vec(),
            dimension_type: "minecraft:the_end".to_owned(),
            dimension_name: "minecraft:world".to_owned(),
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
                data.write_string(32767, &format!("quectocraft {}", VERSION));
                data
            }
        }))?;
        client.send_packet(ClientBoundPacket::PlayerAbilities(0x0d, 0.05, 0.1))?;
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
        Ok(())
    }

}
