use super::state::ServerState;
// use futures::sink::Sink;
use futures::stream::Stream;
use std::task::{Poll, Context};
use std::pin::Pin;
use std::marker::Unpin;
use bytes::{BytesMut, BufMut, Buf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {

    #[error("parse error")]
    Parse,
}

#[derive(Debug)]
pub enum Message {
    Info(u8, u16, u32),
}

#[derive(Debug)]
pub struct Decode {
    buffer: BytesMut,
    state: Option<ServerState>,
}

impl Decode {

    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
            state: None,
        }
    }

}

impl Unpin for Decode{}

impl Stream for Decode {
    type Item = Result<Message, Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut current_state = self.state.as_mut();
        loop {
            match current_state {
                Some(state) => {
                    // match state {
                    //     ServerState::ClientInfo => {

                    //     },
                    //     ServerState::Err => {

                    //     }
                    // }
                },
                None => {
                    if self.buffer.has_remaining() {
                        if ServerState::ClientInfo.eq(&self.buffer.get_u8()) {
                            self.state = Some(ServerState::ClientInfo);
                        }
                    } else {
                        return Poll::Pending;
                    }
                }
            }
        }
    }
}