use super::read_stream::{ReadStream, StreamType as ReadStreamType};
use super::write_stream::{StreamType as WriteStreamType, WriteStream};
use crate::config::Client;
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
    async fn hand_shake(mut stream: TcpStream, decode: &mut Decode) -> Result<Self, Error> {
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
                                    let mode = Self::select_mode(&mask, &client_config)?;

                                    let (read_stream, write_stream) =
                                        Self::select_stream(stream, &mask, &client_config).await?;

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

    /**
     *  通过客户端功能信息与本地配置做比较,
     *  匹配出最低能接受的模式
     */
    fn select_mode(mask: &u16, client_config: &Client) -> Result<Mode, Error> {
        if *mask & Support::Push
            && *mask & Support::Pull
            && *client_config.get_support_push()
            && *client_config.get_support_pull()
        {
            Ok(Mode::PushAndPull)
        } else {
            if *mask & Support::Push && *client_config.get_support_push() {
                Ok(Mode::OnlyPush)
            } else if *mask & Support::Pull && *client_config.get_support_pull() {
                Ok(Mode::OnlyPull)
            } else {
                Err(Error::HandShake)
            }
        }
    }

    async fn select_stream(
        stream: TcpStream,
        mask: &u16,
        client_config: &Client,
    ) -> Result<(ReadStream, WriteStream), Error> {
        if *mask & Support::Tls && client_config.is_support_tls() {
            if let Some(tls_config) = client_config.get_tls_config().as_ref() {
                let key = File::open(tls_config.get_identity_path().clone()).await?;

                let stream = accept(key, tls_config.get_ssl_password().clone(), stream).await?;

                let (read_stream, write_stream) = split(stream);

                Ok((
                    ReadStream::new(ReadStreamType::Tls(read_stream)),
                    WriteStream::new(WriteStreamType::Tls(write_stream)),
                ))
            } else {
                let (read_stream, write_stream) = split(stream);
                Ok((
                    ReadStream::new(ReadStreamType::Normal(read_stream)),
                    WriteStream::new(WriteStreamType::Normal(write_stream)),
                ))
            }
        } else {
            let (read_stream, write_stream) = split(stream);

            Ok((
                ReadStream::new(ReadStreamType::Normal(read_stream)),
                WriteStream::new(WriteStreamType::Normal(write_stream)),
            ))
        }
    }

    pub(super) async fn run(stream: TcpStream) {
        let client_config = CONFIG.get_client_config();
        let mut decode = Decode::new(*client_config.get_max_message_length() as usize);

        match Self::hand_shake(stream, &mut decode).await {
            Ok(mut serivce) => {}
            Err(e) => {
                error!("error {:?}", e);
            }
        }
    }
}
