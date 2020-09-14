use super::state::{ClientState, Support, STATE_SERVER_INFO};
use bytes::{BufMut, BytesMut};
use std::default::Default;
use std::u32::MAX as u32_MAX;

#[derive(Debug)]
pub struct ServerConfig {
    version: u8,
    support: u16,
    max_message_length: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            version: 1,
            support: 0,
            max_message_length: u32_MAX,
        }
    }
}

impl ServerConfig {
    pub fn set_version(&mut self, version: u8) {
        self.version = version;
    }

    pub fn support_push(&mut self) {
        self.support |= Support::Push;
    }

    pub fn support_pull(&mut self) {
        self.support |= Support::Pull;
    }

    pub fn support_tls(&mut self) {
        self.support |= Support::Tls;
    }

    pub fn support_compress(&mut self) {
        self.support |= Support::Compress;
    }

    pub fn max_message_length(&mut self, max_message_length: u32) {
        self.max_message_length = max_message_length;
    }

    pub fn encode(self) -> BytesMut {
        let mut buff = BytesMut::with_capacity(9);

        buff.put_u8(STATE_SERVER_INFO);
        buff.put_u8(self.version);
        buff.put_u16_le(self.support);
        buff.put_u32_le(self.max_message_length);

        buff
    }
}
