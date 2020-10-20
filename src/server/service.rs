use super::read_stream::{ReadStream, StreamType as ReadStreamType};
use super::write_stream::{StreamType as WriteStreamType, WriteStream};
use crate::config::Client;
use crate::global_static::CONFIG;
use async_native_tls::{accept, AcceptError};
use futures::future::FusedFuture;
use futures::stream::StreamExt;
use log::{debug, error};
use protocol::send_to_client::{
    decode::{Decode, Error as DecodeError, Message},
    encode::{Err, Ok, Ping, Pong, ServerConfig},
};
use protocol::state::Support;
use std::io::Error as IoError;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

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

impl Mode {
    fn can_push(&self) -> bool {
        match self {
            Self::OnlyPull => false,
            Self::OnlyPush => true,
            Self::PushAndPull => true,
        }
    }

    fn can_pull(&self) -> bool {
        match self {
            Self::OnlyPull => true,
            Self::OnlyPush => false,
            Self::PushAndPull => true,
        }
    }
}

#[derive(Debug)]
pub(super) struct Service {
    read_stream: ReadStream,
    write_stream: Arc<Mutex<WriteStream>>,
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
                    if let Some(result) = decode.iter().next() {
                        let message = result?;
                        if let Message::Info {
                            version: _version,
                            support: mask,
                            max_message_size: _max_message_size,
                        } = message
                        {
                            let mode = Self::select_mode(&mask, &client_config)?;

                            let (read_stream, write_stream) =
                                Self::select_stream(stream, &mask, &client_config).await?;

                            return Ok(Self {
                                read_stream,
                                write_stream: Arc::new(Mutex::new(write_stream)),
                                mode,
                            });
                        } else {
                            return Err(Error::HandShake);
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
        } else if *mask & Support::Push && *client_config.get_support_push() {
            Ok(Mode::OnlyPush)
        } else if *mask & Support::Pull && *client_config.get_support_pull() {
            Ok(Mode::OnlyPull)
        } else {
            Err(Error::HandShake)
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

    pub(super) async fn run(mut stream: TcpStream) {
        if let Err(e) = stream.set_nodelay(true) {
            error!("stream set nodelay error because {}", e);
        }

        let client_config = CONFIG.get_client_config();
        let mut decode = Decode::new(*client_config.get_max_message_length() as usize);

        match Self::hand_shake(stream, &mut decode).await {
            Ok(mut service) => 'main: loop {
                match service.read_stream.read_buf(decode.get_mut_buffer()).await {
                    Ok(0) => break 'main,
                    Ok(_) => {
                        for result in decode.iter() {
                            match result {
                                Ok(message) => {
                                    if let Err(e) = service.match_message(message).await {
                                        error!("{:?}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("parse error {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("read buff error {:?}", e);
                    }
                }
            },
            Err(e) => {
                error!("error {:?}", e);
            }
        }
    }

    // 匹配消息, 然后做对应的动作
    async fn match_message(&mut self, message: Message) -> Result<(), IoError> {
        debug!("message {:?}", message);
        match message {
            Message::Ping => {
                return self.send_pong().await;
            }
            Message::Pong => {
                return self.send_ping().await;
            }
            Message::TurnPull => {
                return self.send_turn_pull().await;
            }
            Message::TurnPush => {
                return self.send_turn_push().await;
            }
            Message::Sub { name, reply } => {
                self.add_subscribe().await;
            }
            _ => {}
        }
        Ok(())
    }

    // 发送pong消息
    async fn send_pong(&mut self) -> Result<(), IoError> {
        let mut stream = self.write_stream.lock().await;
        stream.write(Pong::encode()).await?;
        stream.flush().await?;
        Ok(())
    }

    // 发送ping消息
    async fn send_ping(&mut self) -> Result<(), IoError> {
        let mut stream = self.write_stream.lock().await;
        stream.write(Ping::encode()).await?;
        stream.flush().await?;
        Ok(())
    }

    // 发送转换pull消息
    async fn send_turn_pull(&mut self) -> Result<(), IoError> {
        let mut stream = self.write_stream.lock().await;
        if self.mode.can_pull() {
            stream.write(Ok::encode()).await?;
        } else {
            stream
                .write(&(Err::new("server not support pull").encode()))
                .await?;
        }

        // 立刻发送
        stream.flush().await?;
        Ok(())
    }

    //发送转换push消息
    async fn send_turn_push(&mut self) -> Result<(), IoError> {
        let mut stream = self.write_stream.lock().await;
        if self.mode.can_push() {
            stream.write(Ok::encode()).await?;
        } else {
            stream
                .write(&(Err::new("server not support push").encode()))
                .await?;
        }

        // 立刻发送
        stream.flush().await?;
        Ok(())
    }

    // 添加订阅
    async fn add_subscribe(&mut self) {}
}
