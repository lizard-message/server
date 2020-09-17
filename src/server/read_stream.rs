use async_native_tls::TlsStream;
use std::io::Result as IoResult;
use std::marker::Unpin;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;
use tokio::io::{BufReader, ReadHalf};
use tokio::net::TcpStream;

#[derive(Debug)]
pub(super) enum StreamType {
    TLS(ReadHalf<TlsStream<TcpStream>>),
    Normal(ReadHalf<TcpStream>),
}

impl Unpin for StreamType {}

impl AsyncRead for StreamType {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        self.poll_read(cx, buf)
    }
}

#[derive(Debug)]
pub(super) struct ReadStream {
    stream: BufReader<StreamType>,
}

impl ReadStream {
    pub(super) fn new(stream: StreamType) -> Self {
        Self {
            stream: BufReader::new(stream),
        }
    }
}
