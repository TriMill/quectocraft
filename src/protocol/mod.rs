
pub mod command;
pub mod data;
pub mod serverbound;
pub mod clientbound;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NetworkState {
    Handshake,
    Status,
    Login,
    Play
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i16,
    pub z: i32
}
