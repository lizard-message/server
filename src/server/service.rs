use super::read_stream::{ReadStream, StreamType as ReadStreamType};
use super::write_stream::{StreamType as WriteStreamType, WriteStream};
use crate::config::Client;
use crate::global_static::CONFIG;
use async_native_tls::{accept, AcceptError};
use bytes::BytesMut;
use log::{debug, error, trace};
use protocol::send_to_client::{
    decode::{Decode, Error as DecodeError, Message},
    encode::{Err, Msg, Ok, Ping, Pong, ServerConfig},
};
use protocol::state::Support;
use radix_trie::Trie;
use std::collections::{HashMap, HashSet};
use std::io::Error as IoError;
use std::ops::Drop;
use std::string::FromUtf8Error;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::runtime::Handle;
use tokio::select;
use tokio::spawn;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::time::{interval, Instant};

#[cfg(unix)]
use std::os::unix::io::RawFd;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
use std::os::windows::io::RawSocket;

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

#[cfg(unix)]
type Fd = RawFd;

#[cfg(windows)]
type Fd = RawSocket;

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

    #[error("bytes convert to utf8 error `{0}`")]
    Utf8(#[from] FromUtf8Error),
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

type ArcSender = Sender<Arc<BytesMut>>;
type ArcShareTrie = Arc<Mutex<Trie<String, HashMap<Fd, ArcSender>>>>;

#[derive(Debug)]
pub(super) struct Service {
    read_stream: ReadStream,
    write_stream: WriteStream,
    mode: Mode,
    share_trie: ArcShareTrie,
    message_offset: Arc<AtomicU64>,
    sender: ArcSender,
    receiver: Receiver<Arc<BytesMut>>,
    sub_name_list: HashSet<String>,
    file_descriptior: Fd,
}

impl Service {
    async fn hand_shake(
        mut stream: TcpStream,
        decode: &mut Decode,
        share_trie: ArcShareTrie,
        message_offset: Arc<AtomicU64>,
    ) -> Result<Self, Error> {
        stream.set_nodelay(false)?;
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
                        if let Message::Info(info) = message {
                            let mode = Self::select_mode(&info.support, &client_config)?;

                            #[cfg(unix)]
                            let file_descriptior = stream.as_raw_fd();

                            #[cfg(windows)]
                            let file_descriptior = stream.as_raw_socket();

                            let (read_stream, write_stream) =
                                Self::select_stream(stream, &info.support, &client_config).await?;

                            let (sender, receiver) = channel(64);

                            return Ok(Self {
                                read_stream,
                                write_stream,
                                mode,
                                share_trie,
                                message_offset,
                                sender,
                                receiver,
                                sub_name_list: HashSet::new(),
                                file_descriptior,
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

    /**
     *  选择使用tls还是普通的流
     */
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

    pub(super) async fn run(
        mut stream: TcpStream,
        share_trie: ArcShareTrie,
        offset: Arc<AtomicU64>,
    ) {
        if let Err(e) = stream.set_nodelay(true) {
            error!("stream set nodelay error because {}", e);
        }

        let client_config = CONFIG.get_client_config();
        let mut decode = Decode::new(*client_config.get_max_message_length() as usize);
        let mut interval = interval(Duration::from_millis(500));

        match Self::hand_shake(stream, &mut decode, share_trie, offset).await {
            Ok(mut service) => 'main: loop {
                select! {
                    result = service.read_stream.read_buf(decode.get_mut_buffer()) => {
                        match result {
                          Ok(0) => break 'main,
                          Ok(_) => {
                              let start_line = line!();
                              let start_time = Instant::now();
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
                              trace!("范围 {}-{} 耗时 {:?}", start_line, line!(),Instant::now().duration_since(start_time));
                          }
                          Err(e) => {
                              error!("read buff error {:?}", e);
                          }
                       }
                    },
                    _ = interval.tick() => {
                        if let Err(e) = service.write_stream.flush().await {
                            error!("flush error {:?}", e);
                        }
                    }
                    msg_option = service.receiver.recv() => {
                        let start_line = line!();
                        let start_time = Instant::now();
                        let msg = msg_option.unwrap();
                        if let Err(e) = service.write_stream.write(&msg).await {
                            error!("publish msg error {:?}", e);
                        }
                        trace!("范围 {}-{} 耗时 {:?}", start_line, line!(),Instant::now().duration_since(start_time));
                    }
                }
            },
            Err(e) => {
                error!("error {:?}", e);
            }
        }
    }

    /**
     *  匹配消息, 然后做对应的动作
     *  短消息需要调用flush方法, 因为使用了BuffWrite, 有可能会缓存了一定的消息而没有发出去
     *  特别是对于心跳的消息, 需要一定的实时
     */
    async fn match_message(&mut self, message: Message) -> Result<(), Error> {
        debug!("message {:?}", message);
        match message {
            Message::Ping => {
                self.send_pong().await?;
            }
            Message::Pong => {
                self.send_ping().await?;
            }
            Message::TurnPull => {
                self.send_turn_pull().await?;
            }
            Message::TurnPush => {
                self.send_turn_push().await?;
            }
            Message::Sub(sub) => {
                let sub_name = String::from_utf8(sub.name.to_vec())?;
                self.add_subscribe(sub_name).await;
            }
            Message::Pub(r#pub) => {
                let sub_name = String::from_utf8(r#pub.name.to_vec())?;
                self.pushlish(sub_name, r#pub.msg).await;
            }
            Message::UnSub(unsub) => {
                let sub_name = String::from_utf8(unsub.name.to_vec())?;
                self.unsub(sub_name).await;
            }
            _ => {}
        }
        Ok(())
    }

    // 发送pong消息
    async fn send_pong(&mut self) -> Result<(), IoError> {
        self.write_stream.write(Pong::encode()).await?;
        self.write_stream.flush().await?;
        Ok(())
    }

    // 发送ping消息
    async fn send_ping(&mut self) -> Result<(), IoError> {
        self.write_stream.write(Ping::encode()).await?;
        self.write_stream.flush().await?;
        Ok(())
    }

    // 发送转换pull消息
    async fn send_turn_pull(&mut self) -> Result<(), IoError> {
        if self.mode.can_pull() {
            self.write_stream.write(Ok::encode()).await?;
        } else {
            self.write_stream
                .write(&(Err::new("server not support pull").encode()))
                .await?;
        }

        // 立刻发送
        self.write_stream.flush().await?;
        Ok(())
    }

    //发送转换push消息
    async fn send_turn_push(&mut self) -> Result<(), IoError> {
        if self.mode.can_push() {
            self.write_stream.write(Ok::encode()).await?;
        } else {
            self.write_stream
                .write(&(Err::new("server not support push").encode()))
                .await?;
        }

        // 立刻发送
        self.write_stream.flush().await?;
        Ok(())
    }

    // 添加订阅
    // 暂时只能使用完整的匹配语句, 没有做匹配规则
    async fn add_subscribe(&mut self, sub_name: String) {
        let start_line = line!();
        let start_time = Instant::now();
        self.sub_name_list.insert(sub_name.clone());
        let mut share_trie = self.share_trie.lock().await;
        if let Some(node) = share_trie.get_mut(&sub_name) {
            debug!("share_trie push stream");
            node.insert(self.file_descriptior, self.sender.clone());
        } else {
            debug!("share_tire insert vec");
            let mut hm = HashMap::new();
            hm.insert(self.file_descriptior, self.sender.clone());
            share_trie.insert(sub_name, hm);
        }
        trace!(
            "范围 {}-{} 耗时 {:?}",
            start_line,
            line!(),
            Instant::now().duration_since(start_time)
        );
    }

    // 发布消息
    async fn pushlish(&mut self, sub_name: String, msg: BytesMut) {
        let start_line = line!();
        let start_time = Instant::now();
        let mut share_trie = self.share_trie.lock().await;
        trace!(
            "范围 {}-{} 耗时 {:?}",
            start_line,
            line!(),
            Instant::now().duration_since(start_time)
        );
        let msg = Arc::new(
            Msg::new(
                self.message_offset.load(Ordering::Acquire),
                sub_name.as_bytes(),
                &msg,
            )
            .encode(),
        );

        let start_line = line!();
        let start_time = Instant::now();
        // 根据订阅名称获取订阅树上面的socket
        // 然后做并行发送消息
        if let Some(node) = share_trie.get_mut(&sub_name) {
            debug!("node {:?}", node);

            for sender in node.values() {
                let msg2 = Arc::clone(&msg);
                let mut sender2 = sender.clone();

                spawn(async move {
                    if sender2.send(msg2).await.is_err() {
                        error!("resource leak out, line on {}", line!());
                    }
                });
            }
        }
        trace!(
            "范围 {}-{} 耗时 {:?}",
            start_line,
            line!(),
            Instant::now().duration_since(start_time)
        );
    }

    // 取消订阅
    async fn unsub(&mut self, sub_name: String) {
        let mut share_trie = self.share_trie.lock().await;

        if let Some(node) = share_trie.get_mut(&sub_name) {
            node.remove(&self.file_descriptior);
        }
        self.sub_name_list.remove(&sub_name);
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let start_line = line!();
        let start_time = Instant::now();

        let handle = Handle::current();

        handle.block_on(async {
            // 删除挂载在前缀树上面的ArcSender
            let mut share_trie = self.share_trie.lock().await;
            self.sub_name_list.iter().for_each(|sub_name| {
                if let Some(node) = share_trie.get_mut(sub_name) {
                    node.remove(&self.file_descriptior);
                }
            });
        });

        trace!(
            "范围 {}-{} 耗时 {:?}",
            start_line,
            line!(),
            Instant::now().duration_since(start_time)
        );
    }
}
