use uuid::Uuid;

use super::{data::{PacketEncoder, finalize_packet}, Position};

pub trait ClientBoundPacket: std::fmt::Debug {
    fn encode(&self, encoder: &mut impl PacketEncoder);
    fn packet_id(&self) -> i32;
}

//////////////////
//              //
//    Status    //
//              //
//////////////////

#[derive(Debug)]
pub struct PingResponse {
    pub data: i64
}

impl ClientBoundPacket for PingResponse {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_long(self.data)
    }

    fn packet_id(&self) -> i32 { 0x01 }
}

#[derive(Debug)]
pub struct StatusResponse {
    pub data: String
}

impl ClientBoundPacket for StatusResponse {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(32767, &self.data)
    }

    fn packet_id(&self) -> i32 { 0x00 }
}


/////////////////
//             //
//    Login    //
//             //
/////////////////

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub name: String,
}

impl ClientBoundPacket for LoginSuccess {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_uuid(self.uuid);
        encoder.write_string(16, &self.name);
        encoder.write_varint(0);
    }
    fn packet_id(&self) -> i32 { 0x02 }
}

#[derive(Debug)]
pub struct LoginPluginRequest {
    pub id: i32, 
    pub channel: String, 
    pub data: Vec<u8>,
}

impl ClientBoundPacket for LoginPluginRequest {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_varint(self.id);
        encoder.write_string(32767, &self.channel);
        encoder.write_bytes(&self.data);
    }

    fn packet_id(&self) -> i32 { 0x04 }
}

#[derive(Debug)]
pub struct LoginDisconnect {
    pub reason: serde_json::Value
}

impl ClientBoundPacket for LoginDisconnect {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(262144, &self.reason.to_string())
    }

    fn packet_id(&self) -> i32 { 0x00 }
}


////////////////
//            //
//    Play    //
//            //
////////////////

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

impl ClientBoundPacket for LoginPlay {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_int(self.eid);
        encoder.write_bool(self.is_hardcore);
        encoder.write_ubyte(self.gamemode);
        encoder.write_ubyte(self.prev_gamemode);
        encoder.write_varint(self.dimensions.len() as i32);
        for dim in &self.dimensions {
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
        if let Some(dl) = &self.death_location {
            encoder.write_string(32767, &dl.0);
            encoder.write_position(dl.1);
        }
    }

    fn packet_id(&self) -> i32 { 0x24 }
}

#[derive(Debug)]
pub struct PluginMessage {
    pub channel: String,
    pub data: Vec<u8>,
}

impl ClientBoundPacket for PluginMessage {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(32767, &self.channel);
        encoder.write_bytes(&self.data);
    }

    fn packet_id(&self) -> i32 { 0x15 }
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

impl ClientBoundPacket for SyncPlayerPosition {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_double(self.x);
        encoder.write_double(self.y);
        encoder.write_double(self.z);
        encoder.write_float(self.yaw);
        encoder.write_float(self.pitch);
        encoder.write_byte(self.flags);
        encoder.write_varint(self.teleport_id);
        encoder.write_bool(self.dismount);
    }

    fn packet_id(&self) -> i32 { 0x38 }
}

#[derive(Debug)]
pub struct ChunkData {
    pub x: i32,
    pub z: i32,
    pub heightmap: nbt::Blob,
    pub chunk_data: Vec<u8>,
}

impl ClientBoundPacket for ChunkData {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
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

    fn packet_id(&self) -> i32 { 0x20 }
}

#[derive(Debug)]
pub struct KeepAlive {
    pub data: i64
}

impl ClientBoundPacket for KeepAlive {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_long(self.data)
    }

    fn packet_id(&self) -> i32 { 0x1f }
}

#[derive(Debug)]
pub struct PlayerAbilities {
    pub flags: i8,
    pub fly_speed: f32,
    pub fov_modifier: f32,
}

impl ClientBoundPacket for PlayerAbilities {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_byte(self.flags);
        encoder.write_float(self.fly_speed);
        encoder.write_float(self.fov_modifier);
    }

    fn packet_id(&self) -> i32 { 0x30 }
}


#[derive(Debug)]
pub struct Disconnect {
    pub reason: serde_json::Value
}

impl ClientBoundPacket for Disconnect {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(262144, &self.reason.to_string())
    }

    fn packet_id(&self) -> i32 { 0x17 }
}

#[derive(Debug)]
pub struct SetDefaultSpawnPosition {
    pub pos: Position,
    pub angle: f32
}

impl ClientBoundPacket for SetDefaultSpawnPosition {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_position(self.pos);
        encoder.write_float(self.angle);
    }

    fn packet_id(&self) -> i32 { 0x4c }
}

#[derive(Debug)]
pub struct SystemChatMessage {
    pub message: serde_json::Value,
    pub overlay: bool
}

impl ClientBoundPacket for SystemChatMessage {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_string(262144, &self.message.to_string());
        encoder.write_bool(self.overlay);
    }

    fn packet_id(&self) -> i32 { 0x60 }
}

pub fn encode_packet(packet: impl ClientBoundPacket) -> Vec<u8> {
    let mut buffer = Vec::new();
    packet.encode(&mut buffer);
    finalize_packet(buffer, packet.packet_id())
}
