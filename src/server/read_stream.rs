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
    Tls(ReadHalf<TlsStream<TcpStream>>),
    Normal(ReadHalf<TcpStream>),
}

impl Unpin for StreamType {}

impl AsyncRead for StreamType {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        let mut this = self.get_mut();
        match this {
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Normal(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

#[derive(Debug)]
pub(super) struct ReadStream {
//    stream: BufReader<StreamType>,
    stream: StreamType,
}

impl ReadStream {
    pub(super) fn new(stream: StreamType) -> Self {
        Self {
//            stream: BufReader::new(stream),
            stream,
        }
    }
}

impl AsyncRead for ReadStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}
