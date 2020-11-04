use lizard::{Error, Server};
// use pretty_env_logger::init;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    fmt::init();

    Server::run().await.unwrap();
}
