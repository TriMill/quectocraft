use std::io::{Write, Read};

use serde::Serialize;
use uuid::Uuid;

use super::Position;

pub trait PacketEncoder: Write {
    fn write_bytes(&mut self, data: &[u8]) {
        self.write_all(data).unwrap();
    }

    fn write_bool(&mut self, data: bool) { 
        self.write_all(&[data as u8]).unwrap();
    }

    fn write_byte(&mut self, data: i8) { 
        self.write_all(&[data as u8]).unwrap();
    }

    fn write_ubyte(&mut self, data: u8) { 
        self.write_all(&[data]).unwrap();
    }

    fn write_short(&mut self, data: i16) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }

    fn write_ushort(&mut self, data: u16) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }

    fn write_int(&mut self, data: i32) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }

    fn write_long(&mut self, data: i64) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }

    fn write_uuid(&mut self, data: Uuid) { 
        self.write_all(&data.as_u128().to_be_bytes()).unwrap();
    }

    fn write_float(&mut self, data: f32) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }
    
    fn write_double(&mut self, data: f64) { 
        self.write_all(&data.to_be_bytes()).unwrap();
    }

    fn write_varint(&mut self, mut data: i32) {
        loop {
            let mut byte = (data & 0b11111111) as u8;
            data >>= 7;
            if data != 0 {
                byte |= 0b10000000;
            }
            self.write_all(&[byte]).unwrap();
            if data == 0 {
                break
            }
        }
    }

    fn write_varlong(&mut self, mut data: i64) {
        loop {
            let mut byte = (data & 0b11111111) as u8;
            data >>= 7;
            if data != 0 {
                byte |= 0b10000000;
            }
            self.write_all(&[byte]).unwrap();
            if data == 0 {
                break
            }
        }
    }

    fn write_position(&mut self, position: Position) {
        self.write_long(
            (((position.x & 0x3ffffff) as i64) << 38)
            | (((position.z & 0x3ffffff) as i64) << 12)
            | (position.y & 0xfff) as i64
        )
    }

    fn write_nbt(&mut self, nbt: &impl Serialize) {
        nbt::to_writer(self, nbt, None).unwrap();
    }

    fn write_string(&mut self, max_len: usize, val: &str) {
        if val.len() > max_len * 4 + 3 {
            panic!("exceeded max string length")
        }
        self.write_varint(val.len() as i32);
        self.write_all(val.as_bytes()).unwrap();
    }

    fn to_data(self) -> Vec<u8>;
}

impl PacketEncoder for Vec<u8> {
    fn to_data(self) -> Vec<u8> { self }
}

fn encode_varint(mut data: i32) -> Vec<u8> {
    let mut res = Vec::new();
    loop {
        let mut byte = (data & 0b11111111) as u8;
        data >>= 7;
        if data != 0 {
            byte |= 0b10000000;
        }
        res.push(byte);
        if data == 0 {
            break
        }
    }
    res
}

pub fn finalize_packet(packet: impl PacketEncoder, packet_id: i32) -> Vec<u8> {
    let mut id = encode_varint(packet_id);
    let mut data = packet.to_data();
    let mut result = encode_varint((id.len() + data.len()) as i32);
    result.append(&mut id);
    result.append(&mut data);
    result
}

pub struct PacketDecoder {
    data: Vec<u8>,
    idx: usize,
    packet_id: i32,
}

#[allow(unused)]
impl PacketDecoder {
    pub fn decode(read: &mut impl Read) -> Result<PacketDecoder, std::io::Error> {
        let size = read_varint(read)? as usize;
        let mut data = vec![0; size];
        read.read_exact(&mut data).unwrap();
        let mut decoder = PacketDecoder {
            data,
            idx: 0,
            packet_id: 0
        };
        decoder.packet_id = decoder.read_varint();
        Ok(decoder)
    }

    pub fn packet_id(&self) -> i32 {
        self.packet_id
    }

    pub fn read_bytes(&mut self, n: usize) -> &[u8] {
        let ret = &self.data[self.idx..self.idx+n];
        self.idx += n;
        ret
    }

    pub fn read_bool(&mut self) -> bool { 
        self.idx += 1;
        self.data[self.idx-1] != 0
    }
    
    pub fn read_byte(&mut self) -> i8 { 
        self.idx += 1;
        self.data[self.idx-1] as i8
    }

    pub fn read_ubyte(&mut self) -> u8 { 
        self.idx += 1;
        self.data[self.idx-1]
    }
    
    pub fn read_short(&mut self) -> i16 { 
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.data[self.idx..self.idx+2]);
        self.idx += 2;
        i16::from_be_bytes(buf)
    }

    pub fn read_ushort(&mut self) -> u16 { 
        let mut buf = [0; 2];
        buf.copy_from_slice(&self.data[self.idx..self.idx+2]);
        self.idx += 2;
        u16::from_be_bytes(buf)
    }
    
    pub fn read_int(&mut self) -> i32 { 
        let mut buf = [0; 4];
        buf.copy_from_slice(&self.data[self.idx..self.idx+4]);
        self.idx += 4;
        i32::from_be_bytes(buf)
    }

    pub fn read_long(&mut self) -> i64 { 
        let mut buf = [0; 8];
        buf.copy_from_slice(&self.data[self.idx..self.idx+8]);
        self.idx += 8;
        i64::from_be_bytes(buf)
    }
    
    pub fn read_uuid(&mut self) -> Uuid { 
        let mut buf = [0; 16];
        buf.copy_from_slice(&self.data[self.idx..self.idx+16]);
        self.idx += 16;
        Uuid::from_u128(u128::from_be_bytes(buf))
    }
    
    pub fn read_float(&mut self) -> f32 { 
        let mut buf = [0; 4];
        buf.copy_from_slice(&self.data[self.idx..self.idx+4]);
        self.idx += 4;
        f32::from_be_bytes(buf)
    }

    pub fn read_double(&mut self) -> f64 { 
        let mut buf = [0; 8];
        buf.copy_from_slice(&self.data[self.idx..self.idx+8]);
        self.idx += 8;
        f64::from_be_bytes(buf)
    }

    pub fn read_varint(&mut self) -> i32 {
        let mut result = 0;
        let mut count = 0;
        loop {
            let byte = self.read_ubyte();
            result |= ((byte & 0x7f) as i32) << (7 * count);
            count += 1;
            if count > 5 {
                panic!("varint too long")
            }
            if byte & 0x80 == 0 {
                break
            }
        }
        result
    }

    pub fn read_varlong(&mut self) -> i64 {
        let mut result = 0;
        let mut count = 0;
        loop {
            let byte = self.read_ubyte();
            result |= ((byte & 0x7f) as i64) << (7 * count);
            count += 1;
            if count > 10 {
                panic!("varint too long")
            }
            if byte & 0x80 == 0 {
                break
            }
        }
        result
    }

    pub fn read_string(&mut self) -> String {
        let len = self.read_varint() as usize;
        String::from_utf8(self.read_bytes(len).to_vec()).unwrap()
    }
}

fn read_varint(read: &mut impl Read) -> Result<i32, std::io::Error> {
    let mut result = 0;
    let mut count = 0;
    loop {
        let mut byte = [0];
        read.read_exact(&mut byte)?;
        let byte = byte[0];
        result |= ((byte & 0x7f) as i32) << (7 * count);
        count += 1;
        if count > 5 {
            panic!("varint too long")
        }
        if byte & 0x80 == 0 {
            break
        }
    }
    Ok(result)
}
