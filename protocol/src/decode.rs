use super::state::ServerState;
use bytes::{Buf, BytesMut};
use futures::stream::Stream;
use std::convert::TryInto;
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
    Info(u8, u16, u8),
    Ping,
    Pong,
    TurnPush,
    TurnPull,
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

    pub fn get_mut_buffer(&mut self) -> &mut BytesMut {
        &mut self.buffer
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
                if let Some(state) = &this.state {
                    match state {
                        ServerState::ClientInfo => {
                            if this.buffer.len() > 3 {
                                this.state = None;
                                return Poll::Ready(Some(Ok(Message::Info(
                                    this.buffer.get_u8(),
                                    this.buffer.get_u16_le(),
                                    this.buffer.get_u8(),
                                ))));
                            } else {
                                return Poll::Ready(None);
                            }
                        }
                        ServerState::Ping => {
                            this.state = None;
                            return Poll::Ready(Some(Ok(Message::Ping)));
                        }
                        ServerState::Pong => {
                            this.state = None;
                            return Poll::Ready(Some(Ok(Message::Pong)));
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
                            this.state = None;
                            return Poll::Ready(Some(Ok(Message::TurnPull)));
                        }
                        ServerState::TurnPush => {
                            this.state = None;
                            return Poll::Ready(Some(Ok(Message::TurnPush)));
                        }
                    }
                } else {
                    let byte = this.buffer.get_u8();

                    match byte.try_into() {
                        Ok(state) => this.state = Some(state),
                        Err(e) => return Poll::Ready(Some(Err(Error::Parse))),
                    }
                }
            }
        }
    }
}
