use uuid::Uuid;

use super::{data::PacketDecoder, NetworkState};

#[derive(Debug)]
pub struct Handshake {
    pub version: i32,
    pub addr: String,
    pub port: u16,
    pub next_state: i32,
}

impl Handshake {
    pub fn decode(mut decoder: PacketDecoder) -> Self {
        let version = decoder.read_varint();
        let addr = decoder.read_string();
        let port = decoder.read_ushort();
        let next_state = decoder.read_varint();
        Self { version, addr, port, next_state }
    }
}

#[derive(Debug)]
pub struct SigData {
    pub timestamp: i64,
    pub pubkey: Vec<u8>,
    pub sig: Vec<u8>,
}

#[derive(Debug)]
pub struct LoginStart {
    pub name: String,
    pub sig_data: Option<SigData>,
    pub uuid: Option<Uuid>,
}

impl LoginStart {
    pub fn decode(mut decoder: PacketDecoder) -> Self {
        let name = decoder.read_string();
        let has_sig_data = decoder.read_bool();
        let sig_data = if has_sig_data {
            let timestamp = decoder.read_long();
            let pubkey_len = decoder.read_varint();
            let pubkey = decoder.read_bytes(pubkey_len as usize).to_vec();
            let sig_len = decoder.read_varint();
            let sig = decoder.read_bytes(sig_len as usize).to_vec();
            Some(SigData { timestamp, pubkey, sig })
        } else {
            None
        };
        let has_uuid = decoder.read_bool();
        let uuid = if has_uuid {
            Some(decoder.read_uuid())
        } else {
            None
        };
        Self { name, sig_data, uuid }
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub message: String,
    pub timestamp: i64,
}

impl ChatMessage {
    pub fn decode(mut decoder: PacketDecoder) -> Self {
        let message = decoder.read_string();
        let timestamp = decoder.read_long();
        // TODO read rest of packet
        Self { message, timestamp }
    }
}

#[derive(Debug)]
pub enum ServerBoundPacket {
    Unknown(i32),
    Ignored(i32),
    // handshake
    Handshake(Handshake),
    // status
    StatusRequest(),
    PingRequest(i64),
    // login
    LoginStart(LoginStart),
    // play
    ChatMessage(ChatMessage),
}

impl ServerBoundPacket {
    pub fn decode(state: &mut NetworkState, mut decoder: PacketDecoder) -> ServerBoundPacket {
        use NetworkState as NS;
        match (*state, decoder.packet_id()) {
            (NS::Handshake, 0) => {
                let hs = Handshake::decode(decoder);
                match hs.next_state {
                    1 => *state = NS::Status,
                    2 => *state = NS::Login,
                    state => panic!("invalid next state: {}", state)
                }
                ServerBoundPacket::Handshake(hs)
            },
            (NS::Status, 0) 
                => ServerBoundPacket::StatusRequest(),
            (NS::Status, 1) 
                => ServerBoundPacket::PingRequest(decoder.read_long()),
            (NS::Login, 0) => {
                *state = NS::Play;
                ServerBoundPacket::LoginStart(LoginStart::decode(decoder))
            },
            (NS::Play, 5) => ServerBoundPacket::ChatMessage(ChatMessage::decode(decoder)),
            (NS::Play, id @ (18 | 20 | 21 | 22 | 30)) => ServerBoundPacket::Ignored(id),
            (_, id) => ServerBoundPacket::Unknown(id),
        }
    }
}
