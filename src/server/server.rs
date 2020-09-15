use super::service::Service;
use crate::global_static::CONFIG;
use std::io::Error as IoError;
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::spawn;
use log::error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Io error `{0}`")]
    Io(#[from] IoError),

    #[error("addr parse `{0}`")]
    AddrParse(#[from] AddrParseError),
}

#[derive(Debug)]
pub struct Server;

impl Server {
    pub async fn run() -> Result<(), Error> {
        let client = CONFIG.get_client_config();
        let host = client.get_host();
        let port = client.get_port();

        let mut listener = TcpListener::bind(SocketAddr::new(host.parse()?, *port)).await?;

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {

                    match Service::new(socket).await {
                        Ok(service) => {
                            spawn(service.run());
                        }
                        Err(e) => {
                            error!("service error {:?}", e);
                        }
                    }
                }
                Err(e) => {}
            }
        }
    }
}
