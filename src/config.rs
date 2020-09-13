use serde_derive::Deserialize;
use std::fs::File;
use std::io::{BufReader, Error as IoError, Read};
use thiserror::Error;
use toml::de::Error as TomlError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("toml deserialize `{0}`")]
    TomlDeserialize(#[from] TomlError),
}

#[derive(Deserialize)]
pub struct Config {
    client: Client,
}

impl Config {
    pub fn new(config_path: &str) -> Result<Self, Error> {
        let file_handle = File::open(config_path)?;
        let mut reader = BufReader::new(file_handle);
        let mut buff = Vec::new();

        reader.read_to_end(&mut buff)?;

        Ok(toml::from_slice(&buff)?)
    }

    pub fn get_client_config(&self) -> &Client {
        &self.client
    }
}

#[derive(Deserialize)]
pub struct Client {
    host: String,
    port: u16,
    support_tls: bool,
    support_push: bool,
    support_pull: bool,
    support_compress: bool,
    max_message_length: u32,
}

impl Client {
    pub fn get_host(&self) -> &String {
        &self.host
    }

    pub fn get_port(&self) -> &u16 {
        &self.port
    }

    pub fn get_support_tls(&self) -> &bool {
        &self.support_tls
    }

    pub fn get_support_push(&self) -> &bool {
        &self.support_push
    }

    pub fn get_support_pull(&self) -> &bool {
        &self.support_pull
    }

    pub fn get_support_compress(&self) -> &bool {
        &self.support_compress
    }

    pub fn get_max_message_length(&self) -> &u32 {
        &self.max_message_length
    }
}
