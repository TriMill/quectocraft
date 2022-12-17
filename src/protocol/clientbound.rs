use uuid::Uuid;

use super::{data::{PacketEncoder, finalize_packet}, Position, command::Commands};

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub name: String,
}

impl LoginSuccess {
    pub fn encode(self, encoder: &mut impl PacketEncoder) {
        encoder.write_uuid(self.uuid);
        encoder.write_string(16, &self.name);
        encoder.write_varint(0);
    }
}

#[derive(Debug)]
pub struct LoginPlay {
    pub eid: i32,
    pub is_hardcore: bool,
    pub gamemode: u8,
    pub prev_gamemode: u8,
    pub dimensions: Vec<String>,
    pub registry_codec: Vec<u8>,
    pub dimension_type: String,
    pub dimension_name: String,
    pub seed_hash: i64,
    pub max_players: i32,
    pub view_distance: i32,
    pub sim_distance: i32,
    pub reduced_debug_info: bool,
    pub respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death_location: Option<(String, Position)>
}

impl LoginPlay {
    pub fn encode(self, encoder: &mut impl PacketEncoder) {
        encoder.write_int(self.eid);
        encoder.write_bool(self.is_hardcore);
        encoder.write_ubyte(self.gamemode);
        encoder.write_ubyte(self.prev_gamemode);
        encoder.write_varint(self.dimensions.len() as i32);
        for dim in self.dimensions {
            encoder.write_string(32767, &dim);
        }
        encoder.write_bytes(&self.registry_codec);
        encoder.write_string(32767, &self.dimension_type);
        encoder.write_string(32767, &self.dimension_name);
        encoder.write_long(self.seed_hash);
        encoder.write_varint(self.max_players);
        encoder.write_varint(self.view_distance);
        encoder.write_varint(self.sim_distance);
        encoder.write_bool(self.reduced_debug_info);
        encoder.write_bool(self.respawn_screen);
        encoder.write_bool(self.is_debug);
        encoder.write_bool(self.is_flat);
        encoder.write_bool(self.death_location.is_some());
        if let Some(dl) = self.death_location {
            encoder.write_string(32767, &dl.0);
            encoder.write_position(dl.1);
        }
    }
}

#[derive(Debug)]
pub struct PluginMessage {
    pub channel: String,
    pub data: Vec<u8>,
}

impl PluginMessage {
    pub fn encode(self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(32767, &self.channel);
        encoder.write_bytes(&self.data);
    }
}

#[derive(Debug)]
pub struct SyncPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: i8,
    pub teleport_id: i32,
    pub dismount: bool,
}

impl SyncPlayerPosition {
    pub fn encode(self, encoder: &mut impl PacketEncoder) {
        encoder.write_double(self.x);
        encoder.write_double(self.y);
        encoder.write_double(self.z);
        encoder.write_float(self.yaw);
        encoder.write_float(self.pitch);
        encoder.write_byte(self.flags);
        encoder.write_varint(self.teleport_id);
        encoder.write_bool(self.dismount);
    }
}

#[derive(Debug)]
pub struct ChunkData {
    pub x: i32,
    pub z: i32,
    pub heightmap: nbt::Blob,
    pub chunk_data: Vec<u8>,
}

impl ChunkData {
    pub fn encode(self, encoder: &mut impl PacketEncoder) {
        encoder.write_int(self.x);
        encoder.write_int(self.z);
        self.heightmap.to_writer(encoder).unwrap();
        encoder.write_varint(self.chunk_data.len() as i32);
        encoder.write_bytes(&self.chunk_data);
        // number of block entities
        encoder.write_varint(0);
        // trust edges
        encoder.write_bool(true);
        // light masks
        encoder.write_varint(0);
        encoder.write_varint(0);
        encoder.write_varint(0);
        encoder.write_varint(0);
        // sky light array
        encoder.write_varint(0);
        // block light array
        encoder.write_varint(0);
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum ClientBoundPacket {
    // status
    StatusResponse(String),
    PingResponse(i64),
    // login
    LoginPluginRequest { id: i32, channel: String, data: Vec<u8> },
    LoginSuccess(LoginSuccess),
    LoginDisconnect(serde_json::Value),
    // play
    LoginPlay(LoginPlay),
    PluginMessage(PluginMessage),
    Commands(Commands),
    ChunkData(ChunkData),
    SyncPlayerPosition(SyncPlayerPosition),
    KeepAlive(i64),
    PlayerAbilities(i8, f32, f32),
    Disconnect(serde_json::Value),
    SetDefaultSpawnPosition(Position, f32),
    SystemChatMessage(serde_json::Value, bool),
}

impl ClientBoundPacket {
    pub fn encode(self) -> Vec<u8> {
        let mut packet = Vec::new();
        match self {
            // Status
            Self::StatusResponse(status) => {
                packet.write_string(32767, &status);
                finalize_packet(packet, 0)
            },
            Self::PingResponse(n) => {
                packet.write_long(n);
                finalize_packet(packet, 1)
            },
            // Login
            Self::LoginDisconnect(message) => {
                packet.write_string(262144, &message.to_string());
                finalize_packet(packet, 0)
            }
            Self::LoginPluginRequest { id, channel, data } => {
                packet.write_varint(id);
                packet.write_string(32767, &channel);
                packet.write_bytes(&data);
                finalize_packet(packet, 4)
            }
            Self::LoginSuccess(login_success) => {
                login_success.encode(&mut packet);
                finalize_packet(packet, 2)
            }
            // Play
            Self::Disconnect(message) => {
                packet.write_string(262144, &message.to_string());
                finalize_packet(packet, 23)
            }
            Self::LoginPlay(login_play) => {
                login_play.encode(&mut packet);
                finalize_packet(packet, 36)
            }
            Self::PluginMessage(plugin_message) => {
                plugin_message.encode(&mut packet);
                finalize_packet(packet, 21)
            }
            Self::Commands(commands) => {
                commands.encode(&mut packet);
                finalize_packet(packet, 14)
            }
            Self::ChunkData(chunk_data) => {
                chunk_data.encode(&mut packet);
                finalize_packet(packet, 32)
            }
            Self::SyncPlayerPosition(sync_player_position) => {
                sync_player_position.encode(&mut packet);
                finalize_packet(packet, 56)
            }
            Self::KeepAlive(n) => {
                packet.write_long(n);
                finalize_packet(packet, 31)
            }
            Self::SetDefaultSpawnPosition(pos, angle) => {
                packet.write_position(pos);
                packet.write_float(angle);
                finalize_packet(packet, 76)
            }
            Self::PlayerAbilities(flags, speed, view) => {
                packet.write_byte(flags);
                packet.write_float(speed);
                packet.write_float(view);
                finalize_packet(packet, 48)
            }
            Self::SystemChatMessage(msg, overlay) => {
                packet.write_string(262144, &msg.to_string());
                packet.write_bool(overlay);
                finalize_packet(packet, 96)
            }
        }
    }
}
