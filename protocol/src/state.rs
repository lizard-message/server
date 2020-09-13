use std::ops::{BitOrAssign, BitAnd};
use std::convert::Into;
use std::cmp::PartialEq;

pub(crate) const STATE_SERVER_INFO: u8 = 0;
pub(crate) const STATE_CLIENT_INFO: u8 = 1;
pub(crate) const STATE_PING: u8 = 2;
pub(crate) const STATE_PONG: u8 = 3;
pub(crate) const STATE_MSG: u8 = 4;
pub(crate) const STATE_OFFSET: u8 = 5;
pub(crate) const STATE_ACK: u8 = 6;
pub(crate) const STATE_SUB: u8 = 7;
pub(crate) const STATE_UNSUB: u8 = 8;
pub(crate) const STATE_ERR: u8 = 9;
pub(crate) const STATE_TURN_PUSH: u8 = 10;
pub(crate) const STATE_TURN_PULL: u8 = 11;

#[repr(u8)]
#[derive(Debug)]
pub(super) enum ServerState {
    ClientInfo = STATE_CLIENT_INFO,
    Ping = STATE_PING,
    Pong = STATE_PONG,
    Msg = STATE_MSG,
    Offset = STATE_OFFSET,
    Ack = STATE_ACK,
    Err = STATE_ERR,
    TurnPush = STATE_TURN_PUSH,
    TurnPull = STATE_TURN_PULL,
}

impl Into<u8> for ServerState {
    fn into(self) -> u8 {
        match self {
            Self::ClientInfo => STATE_CLIENT_INFO,
            Self::Ping => STATE_PING,
            Self::Pong => STATE_PONG,
            Self::Msg => STATE_MSG,
            Self::Offset => STATE_OFFSET,
            Self::Ack => STATE_ACK,
            Self::Err => STATE_ERR,
            Self::TurnPush => STATE_TURN_PUSH,
            Self::TurnPull => STATE_TURN_PULL,
        }
    }
}

impl PartialEq<u8> for ServerState {

    fn eq(&self, other: &u8) -> bool {
        *self == *other
    }
}

#[repr(u8)]
#[derive(Debug)]
pub(super) enum ClientState {
    ServerInfo = STATE_SERVER_INFO,
    Ping = STATE_PING,
    Pong = STATE_PONG,
    Msg = STATE_MSG,
    Offset = STATE_OFFSET,
    Ack = STATE_ACK,
    Sub = STATE_SUB,
    UnSub = STATE_UNSUB,
    Err = STATE_ERR,
    TurnPush = STATE_TURN_PUSH,
    TurnPull = STATE_TURN_PULL,
}

const SUPPORT_PUSH: u16 = 1;
const SUPPORT_PULL: u16 = 2;
const SUPPORT_TLS: u16 = 4;
const SUPPORT_COMPRESS: u16 = 8;

#[repr(u16)]
#[derive(Debug)]
pub(super) enum Support {
    Push = SUPPORT_PUSH,
    Pull = SUPPORT_PULL,
    Tls = SUPPORT_TLS,
    Compress = SUPPORT_COMPRESS,
}

impl BitOrAssign<Support> for u16 {
    fn bitor_assign(&mut self, rhs: Support) {
        *self |= rhs;
    }
}

impl BitAnd<Support> for u16 {
    type Output = bool;
    fn bitand(self, rhs: Support) -> Self::Output {
        self & rhs
    }
}