use super::read_stream::{ReadStream, StreamType as ReadStreamType};
use super::write_stream::{StreamType as WriteStreamType, WriteStream};
use crate::global_static::CONFIG;
use async_native_tls::{accept, AcceptError};
use futures::stream::StreamExt;
use log::{error, info};
use protocol::decode::{Decode, Error as DecodeError, Message};
use protocol::encode::ServerConfig;
use protocol::state::Support;
use std::io::Error as IoError;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub(super) enum Error {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("hand shake error")]
    HandShake,

    #[error("tls accept error `{0}`")]
    Tls(#[from] AcceptError),

    #[error("decode error `{0}`")]
    Decode(#[from] DecodeError),
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
    pub(super) async fn new(mut stream: TcpStream) -> Result<Self, Error> {
        let client_config = CONFIG.get_client_config();

        let mut server_config = ServerConfig::default();

        if *client_config.get_support_push() {
            server_config.support_push();
        }

        if *client_config.get_support_pull() {
            server_config.support_pull();
        }

        if client_config.is_support_tls() {
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
                }
                Ok(_) => {
                    if let Some(result) = decode.next().await {
                        match result {
                            Ok(message) => {
                                if let Message::Info(version, mask, max_message_size) = message {
                                    let mode = {
                                        if mask & Support::Push
                                            && mask & Support::Pull
                                            && *client_config.get_support_push()
                                            && *client_config.get_support_pull()
                                        {
                                            Mode::PushAndPull
                                        } else {
                                            if mask & Support::Push
                                                && *client_config.get_support_push()
                                            {
                                                Mode::OnlyPush
                                            } else if mask & Support::Pull
                                                && *client_config.get_support_pull()
                                            {
                                                Mode::OnlyPull
                                            } else {
                                                return Err(Error::HandShake);
                                            }
                                        }
                                    };

                                    let (read_stream, write_stream) = {
                                        if mask & Support::Tls && client_config.is_support_tls() {
                                            if let Some(tls_config) =
                                                client_config.get_tls_config().as_ref()
                                            {
                                                let key = File::open(
                                                    tls_config.get_identity_path().clone(),
                                                )
                                                .await?;

                                                let stream = accept(
                                                    key,
                                                    tls_config.get_ssl_password().clone(),
                                                    stream,
                                                )
                                                .await?;

                                                let (read_stream, write_stream) = split(stream);

                                                (
                                                    ReadStream::new(ReadStreamType::Tls(
                                                        read_stream,
                                                    )),
                                                    WriteStream::new(WriteStreamType::Tls(
                                                        write_stream,
                                                    )),
                                                )
                                            } else {
                                                let (read_stream, write_stream) = split(stream);
                                                (
                                                    ReadStream::new(ReadStreamType::Normal(
                                                        read_stream,
                                                    )),
                                                    WriteStream::new(WriteStreamType::Normal(
                                                        write_stream,
                                                    )),
                                                )
                                            }
                                        } else {
                                            let (read_stream, write_stream) = split(stream);

                                            (
                                                ReadStream::new(ReadStreamType::Normal(
                                                    read_stream,
                                                )),
                                                WriteStream::new(WriteStreamType::Normal(
                                                    write_stream,
                                                )),
                                            )
                                        }
                                    };

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
                                return Err(e.into());
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
