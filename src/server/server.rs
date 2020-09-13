use super::service::Service;
use crate::global_static::CONFIG;
use std::io::Error as IoError;
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::spawn;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Io error `{0}`")]
    Io(#[from] IoError),

    #[error("addr parse `{0}`")]
    AddrParse(#[from] AddrParseError),
}

#[derive(Debug)]
pub(crate) struct Server;

impl Server {
    pub(crate) async fn run() -> Result<(), Error> {
        let client = CONFIG.get_client_config();
        let host = client.get_host();
        let port = client.get_port();

        let mut listener = TcpListener::bind(SocketAddr::new(host.parse()?, *port)).await?;

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let service = Service::new(socket).await?;
                    spawn(service.run());
                }
                Err(e) => {}
            }
        }

        Ok(())
    }
}
