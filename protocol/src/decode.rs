use super::state::ServerState;
// use futures::sink::Sink;
use bytes::{Buf, BufMut, BytesMut};
use futures::stream::Stream;
use std::marker::Unpin;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error")]
    Parse,
}

#[derive(Debug)]
pub enum Message {
    Info(u8, u16),
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

impl Unpin for Decode {}

impl Stream for Decode {
    type Item = Result<Message, Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.get_mut();

        loop {
            if !this.buffer.has_remaining() {
                return Poll::Ready(None);
            } else {
                if let Some(state) = this.state.as_mut() {
                    match state {
                        ServerState::ClientInfo => {
                            if this.buffer.len() > 3 {
                                return Poll::Ready(Some(Ok(Message::Info(
                                    this.buffer.get_u8(),
                                    this.buffer.get_u16_le(),
                                ))));
                            } else {
                                return Poll::Ready(None);
                            }
                        }
                        ServerState::Ping => {
                            return Poll::Pending;
                        }
                        ServerState::Pong => {
                            return Poll::Pending;
                        }
                        ServerState::Err => {
                            return Poll::Pending;
                        }
                        ServerState::Msg => {
                            return Poll::Pending;
                        }
                        ServerState::Ack => {
                            return Poll::Pending;
                        }
                        ServerState::Offset => {
                            return Poll::Pending;
                        }
                        ServerState::TurnPull => {
                            return Poll::Pending;
                        }
                        ServerState::TurnPush => {
                            return Poll::Pending;
                        }
                    }
                } else {
                    if ServerState::ClientInfo == this.buffer.get_u8() {
                        this.state = Some(ServerState::ClientInfo);
                    }
                }
            }
        }
    }
}
