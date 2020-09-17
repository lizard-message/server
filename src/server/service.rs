use super::read_stream::{ReadStream, StreamType};
use super::write_stream::WriteStream;
use crate::global_static::CONFIG;
use protocol::encode::ServerConfig;
use protocol::decode::{Decode, Error as DecodeError, Message};
use protocol::state::Support;
use std::io::Error as IoError;
use thiserror::Error;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use futures::stream::StreamExt;
use log::{error, info};
use async_native_tls::accept;

#[derive(Debug, Error)]
pub(super) enum Error {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("hand shake error")]
    HandShake,
}

#[derive(Debug)]
enum Mode {
    OnlyPush,
    OnlyPull,
    PushAndPull,
}

#[derive(Debug)]
pub(super) struct Service {
    read_stream: ReadStream,
    write_stream: WriteStream,
    mode: Mode,
}

impl Service {
    pub(super) async fn new(stream: TcpStream) -> Result<Self, Error> {
        let client_config = CONFIG.get_client_config();
        
        let mut server_config = ServerConfig::default();

        if *client_config.get_support_push() {
            server_config.support_push();
        }

        if *client_config.get_support_pull() {
            server_config.support_pull();
        }

        if *client_config.get_support_tls() {
            server_config.support_tls();
        }

        if *client_config.get_support_compress() {
            server_config.support_compress();
        }

        server_config.max_message_length(*client_config.get_max_message_length());

        stream.write(&server_config.encode().freeze()).await?;

        let mut decode = Decode::new(*client_config.get_max_message_length() as usize);

        loop {
            match stream.read_buf(decode.get_mut_buffer()).await {
                Ok(0) => {
                    return Err(Error::HandShake);
                },
                Ok(_) => {
                    if let Some(result) = decode.next().await {
                        match result {
                            Ok(message) => {
                                if let Message::Info(version, mask, max_message_size) = message {

                                    let mode = {
                                        if mask & Support::Push && mask & Support::Pull && *client_config.get_support_push() && *client_config.get_support_pull() {
                                            Mode::PushAndPull
                                        } else {
                                            if mask & Support::Push && *client_config.get_support_push() {
                                                Mode::OnlyPush
                                            } else if mask & Support::Pull && *client_config.get_support_pull() {
                                                Mode::OnlyPull
                                            } else {
                                                return Err(Error::HandShake);
                                            }
                                        }
                                    };

                                    let (read_stream, write_stream) = {

                                        if mask & Support::Tls && *client_config.get_support_tls() {
                                            accept(, ,stream).await?;
                                        } else {
                                            split(stream)
                                        }
                                    };
                                    let mut read_stream = ReadStream::new(read_stream);
                                    let mut write_stream = WriteStream::new(wirte_stream);

                                    return Ok(Self {
                                        read_stream,
                                        write_stream,
                                        mode,
                                    });
                                } else {
                                    error!("message error: not client info in hand shake");
                                }
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    } else {
                        continue;
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    pub(super) async fn run(mut self) {}
}
