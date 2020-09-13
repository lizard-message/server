use async_native_tls;
use tokio::io::{BufWriter, WriteHalf};
use tokio::net::TcpStream;

#[derive(Debug)]
pub(super) struct WriteStream {
    stream: BufWriter<WriteHalf<TcpStream>>,
}

impl WriteStream {
    pub(super) fn new(stream: WriteHalf<TcpStream>) -> Self {
        Self {
            stream: BufWriter::new(stream),
        }
    }
}
