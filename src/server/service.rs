use super::read_stream::{ReadStream, StreamType};
use super::write_stream::WriteStream;
use crate::global_static::CONFIG;
use protocol::encode::ServerConfig;
use std::io::Error as IoError;
use thiserror::Error;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub(super) enum Error {
    #[error("io error `{0}`")]
    Io(#[from] IoError),
}

#[derive(Debug)]
pub(super) struct Service {
    read_stream: ReadStream,
    write_stream: WriteStream,
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

        stream.write(server_config.encode().as_slice()).await?;

        let (mut read_stream, mut write_stream) = split(stream);
        let mut read_stream = ReadStream::new(read_stream);
        let mut write_stream = WriteStream::new(wirte_stream);

        Ok(Self {
            read_stream,
            write_stream,
        })
    }

    pub(super) async fn run(mut self) {}
}
