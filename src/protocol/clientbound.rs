use uuid::Uuid;

use super::{data::{PacketEncoder, finalize_packet}, Position};

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

#[derive(Debug)]
pub enum ClientBoundPacket {
    // status
    StatusResponse(String),
    PingResponse(i64),
    // login
    LoginSuccess(LoginSuccess),
    // play
    LoginPlay(LoginPlay),
    PluginMessage(PluginMessage),
    SyncPlayerPosition(SyncPlayerPosition),
    ChunkData(ChunkData),
    KeepAlive(i64),
    PlayerAbilities(i8, f32, f32),
    SystemChatMessage(serde_json::Value, bool),
}

impl ClientBoundPacket {
    pub fn encode(self) -> Vec<u8> {
        let mut packet = Vec::new();
        match self {
            Self::StatusResponse(status) => {
                packet.write_string(32767, &status);
                finalize_packet(packet, 0)
            },
            Self::PingResponse(n) => {
                packet.write_long(n);
                finalize_packet(packet, 1)
            },
            Self::LoginSuccess(login_success) => {
                login_success.encode(&mut packet);
                finalize_packet(packet, 2)
            }
            Self::PluginMessage(plugin_message) => {
                plugin_message.encode(&mut packet);
                finalize_packet(packet, 22)
            }
            Self::SyncPlayerPosition(sync_player_position) => {
                sync_player_position.encode(&mut packet);
                finalize_packet(packet, 57)
            }
            Self::LoginPlay(login_play) => {
                login_play.encode(&mut packet);
                finalize_packet(packet, 37)
            }
            Self::ChunkData(chunk_data) => {
                chunk_data.encode(&mut packet);
                finalize_packet(packet, 33)
            }
            Self::KeepAlive(n) => {
                packet.write_long(n);
                finalize_packet(packet, 32)
            }
            Self::PlayerAbilities(flags, speed, view) => {
                packet.write_byte(flags);
                packet.write_float(speed);
                packet.write_float(view);
                finalize_packet(packet, 49)
            }
            Self::SystemChatMessage(msg, overlay) => {
                packet.write_string(262144, &msg.to_string());
                packet.write_bool(overlay);
                finalize_packet(packet, 98)
            }
        }
    }
}
