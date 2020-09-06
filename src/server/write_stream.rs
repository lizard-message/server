use tokio::net::TcpStream;
use tokio::io::ReadHalf;

#[derive(Debug)]
pub(super) struct ReadStream {
    stream: ReadHalf<TcpStream>,
}