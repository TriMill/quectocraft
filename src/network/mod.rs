use std::{net::{TcpStream, TcpListener, Shutdown}, thread, io::Write, sync::mpsc::{Receiver, Sender, channel, TryRecvError}, time::Duration};

use serde_json::json;
use uuid::Uuid;

use crate::{protocol::{data::{PacketDecoder, PacketEncoder}, serverbound::*, clientbound::*, NetworkState}, plugins::Plugins};

pub struct NetworkServer<'lua> {
    plugins: Plugins<'lua>,
    new_clients: Receiver<NetworkClient>,
    clients: Vec<NetworkClient>,
}

impl <'lua> NetworkServer<'lua> {
    pub fn new(addr: String, plugins: Plugins<'lua>) -> Self {
        let (send, recv) = channel();
        thread::spawn(move || Self::listen(&addr, send));
        plugins.init();
        Self { 
            plugins,
            new_clients: recv,
            clients: Vec::new(),
        }
    }

    fn listen(addr: &str, send_clients: Sender<NetworkClient>) {
        println!("listening for connections");
        let listener = TcpListener::bind(addr).unwrap();
        for (id, stream) in listener.incoming().enumerate() {
            let stream = stream.unwrap();
            println!("got connection from {} (id {})", stream.peer_addr().unwrap(), id);
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
        let mut closed = Vec::new();
        for i in 0..self.clients.len() {
            let client: &mut NetworkClient = unsafe {
                &mut *(self.clients.get_unchecked_mut(i) as *mut _)
            };
            let mut alive = true;
            while let Some(packet) = client.recv_packet(&mut alive) {
                if let Err(_) = self.handle_packet(client, packet) {
                    alive = false;
                    break;
                }
            }
            if !alive && !client.closed {
                closed.push(client.id);
                if let Some(pl) = &client.player {
                    self.plugins.player_leave(pl);
                }
                client.close();
            }
        }
        self.clients.retain(|x| !closed.contains(&x.id));
    }

    fn handle_packet(&mut self, client: &mut NetworkClient, packet: ServerBoundPacket) -> std::io::Result<()> {
        match packet {
            ServerBoundPacket::Ignored(_) => (),
            ServerBoundPacket::Unknown(id) => println!("unknown: {}", id),
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
                data.write_string(32767, "QuectoCraft");
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

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub uuid: Uuid,
}

struct NetworkClient {
    pub id: i32,
    pub play: bool,
    pub closed: bool,
    stream: TcpStream,    
    serverbound: Receiver<ServerBoundPacket>,
    player: Option<Player>,
}

impl NetworkClient {
    pub fn listen(mut stream: TcpStream, send: Sender<ServerBoundPacket>) {
        let mut state = NetworkState::Handshake;
        let dur = Duration::from_millis(5);
        loop {
            if let Some(decoder) = PacketDecoder::decode(&mut stream) {
                let packet = ServerBoundPacket::decode(&mut state, decoder);
                send.send(packet).unwrap();
            }
            thread::sleep(dur)
        }
    }

    pub fn recv_packet(&mut self, alive: &mut bool) -> Option<ServerBoundPacket> {
        match self.serverbound.try_recv() {
            Ok(packet) => Some(packet),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                *alive = false;
                None
            }
        }
    }

    pub fn send_packet(&mut self, packet: ClientBoundPacket) -> std::io::Result<()> {
        self.stream.write_all(&packet.encode())?;
        Ok(())
    }

    pub fn close(&mut self) {
        println!("closed connection with {}", self.id);
        let _ = self.stream.shutdown(Shutdown::Both);
        self.closed = true;
    }
}
