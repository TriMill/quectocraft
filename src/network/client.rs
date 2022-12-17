use std::{net::{TcpStream, Shutdown}, thread, io::Write, sync::mpsc::{Receiver, Sender, TryRecvError}, time::Duration};

use log::debug;

use crate::{protocol::{data::{PacketDecoder}, serverbound::*, clientbound::*, NetworkState}};

use super::Player;

pub struct NetworkClient {
    pub id: i32,
    pub verified: bool,
    pub closed: bool,
    pub stream: TcpStream,    
    pub serverbound: Receiver<ServerBoundPacket>,
    pub player: Option<Player>,
}

impl NetworkClient {
    pub fn listen(mut stream: TcpStream, send: Sender<ServerBoundPacket>) {
        let mut state = NetworkState::Handshake;
        let dur = Duration::from_millis(5);
        loop {
            match PacketDecoder::decode(&mut stream) {
                Ok(decoder) => {
                    let packet = ServerBoundPacket::decode(&mut state, decoder);
                    send.send(packet).unwrap();
                }
                Err(_) => break
            }
            thread::sleep(dur)
        }
        let _ = stream.shutdown(Shutdown::Both);
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
        debug!("Closed connection id {}", self.id);
        let _ = self.stream.shutdown(Shutdown::Both);
        self.closed = true;
    }
}
