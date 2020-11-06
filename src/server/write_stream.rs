use async_native_tls::TlsStream;
use std::io::Result as IoResult;
use std::marker::Unpin;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncWrite, BufWriter, WriteHalf};
use tokio::net::TcpStream;

#[derive(Debug)]
pub(super) enum StreamType {
    Tls(WriteHalf<TlsStream<TcpStream>>),
    Normal(WriteHalf<TcpStream>),
}

impl Unpin for StreamType {}

impl AsyncWrite for StreamType {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        let this = self.get_mut();
        match this {
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Normal(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        let this = self.get_mut();
        match this {
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
            Self::Normal(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        let this = self.get_mut();
        match this {
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Normal(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

#[derive(Debug)]
pub(super) struct WriteStream {
    stream: BufWriter<StreamType>,
}

impl WriteStream {
    pub(super) fn new(stream: StreamType) -> Self {
        Self {
            stream: BufWriter::new(stream),
        }
    }
}

impl AsyncWrite for WriteStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        let me = self.get_mut();
        if !me.stream.buffer().is_empty() {
            Pin::new(&mut me.stream).poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<IoResult<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}
