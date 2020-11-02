use super::service::Service;
use crate::global_static::CONFIG;
use log::{debug, error};
use radix_trie::Trie;
use std::io::Error as IoError;
use std::net::{AddrParseError, SocketAddr};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::spawn;
use tokio::sync::Mutex;

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

        let share_trie = Arc::new(Mutex::new(Trie::new()));
        let offset = Arc::new(AtomicU64::new(0));

        let mut listener = TcpListener::bind(SocketAddr::new(host.parse()?, *port)).await?;

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    debug!("ip {:?} connect", addr);
                    spawn(Service::run(
                        socket,
                        Arc::clone(&share_trie),
                        Arc::clone(&offset),
                    ));
                }
                Err(e) => {
                    error!("tcp listen accept error {:?}", e);
                }
            }
        }
    }
}
