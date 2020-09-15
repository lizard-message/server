use pretty_env_logger::init;
use lizard::{Server, Error};

#[tokio::main]
async fn main() {
    init();

    Server::run().await;
}
