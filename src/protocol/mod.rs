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
    x: i32,
    y: i16,
    z: i32
}
