use uuid::Uuid;

mod client;
mod server;

pub use server::NetworkServer;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub uuid: Uuid,
}
