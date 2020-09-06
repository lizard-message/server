use tokio::net::TcpStream;
use tokio::io::WriteHalf;

#[derive(Debug)]
pub(super) struct WriteStream {
    stream: WriteHalf<TcpStream>,
}