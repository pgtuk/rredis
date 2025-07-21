mod connection;
mod redis;
mod server;

use connection::Connection;
use server::Server;

// TODO: read it from .env at some point
const CONNECTION_BUFFER_SIZE: usize = 4096;

#[tokio::main]
async fn main() {
    let server = Server::setup("127.0.0.1:6379", CONNECTION_BUFFER_SIZE);

    if let Err(e) = server.run().await {
        eprintln!("Runtime error = {:?}", e);
    };
}
