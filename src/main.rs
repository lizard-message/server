use lizard::{Error, Server};
use pretty_env_logger::init;

#[tokio::main]
async fn main() {
    init();

    Server::run().await;
}
