use anyhow::Result;
use tokio::net::TcpListener;

use crate::redis::ConnectionHandler;
use crate::redis::Storage;
use crate::Connection;

pub(crate) struct Server {
    addr: &'static str,
    buffer_size: usize,
    storage: Storage,
}

impl Server {
    pub fn setup(addr: &'static str, buffer_size: usize) -> Server {
        Server {
            addr: addr,
            buffer_size: buffer_size,
            storage: Storage::setup(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        // TODO: add gracefull shutdown
        let listener = TcpListener::bind(self.addr).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            let connection = Connection::new(socket, self.buffer_size);
            let mut handler = ConnectionHandler::new(connection, self.storage.clone());

            tokio::spawn(async move { handler.run().await });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        task::JoinHandle,
    };

    async fn run_server(addr: &'static str) -> Result<JoinHandle<Result<()>>> {
        let server = Server::setup(addr, 1024);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _ = ready_tx.send(());
            server.run().await
        });
        ready_rx.await.ok();
        tokio::task::yield_now().await;
        match handle.is_finished() {
            true => match handle.await.err() {
                Some(err) => Err(anyhow!(err)),
                _ => Err(anyhow!("maybe port is in use")),
            },
            false => Ok(handle),
        }
    }

    #[tokio::test]
    async fn test_ping_pong() -> Result<(), Box<dyn std::error::Error>> {
        let addr = "127.0.0.1:6379";
        let server_handler = run_server(addr)
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        let mut socket = TcpStream::connect(addr).await.unwrap();
        let mut buf = Vec::with_capacity(16);
        socket.write_all(b"*1\r\n$4\r\nPING\r\n").await.unwrap();
        socket.read_buf(&mut buf).await.unwrap();
        assert_eq!(&buf, b"+PONG\r\n");
        buf.clear();

        //check consecutive requests
        socket
            .write_all(b"*2\r\n$4\r\nECHO\r\n$3\r\nhey\r\n")
            .await
            .unwrap();
        socket.read_buf(&mut buf).await.unwrap();
        assert_eq!(&buf, b"$3\r\nhey\r\n");

        server_handler.abort();
        Ok(())
    }

    #[tokio::test]
    async fn test_get_set() -> Result<(), Box<dyn std::error::Error>> {
        let addr = "127.0.0.1:6379";
        let server_handler = run_server(addr)
            .await
            .map_err(|e| format!("Failed to start server: {}", e))?;

        let mut socket = TcpStream::connect(addr).await.unwrap();
        let mut buf = Vec::with_capacity(16);

        // get missing key
        let get_input = b"*2\r\n$3\r\nGET\r\n$3\r\nhey\r\n";
        socket.write_all(get_input).await.unwrap();
        socket.read_buf(&mut buf).await.unwrap();
        assert_eq!(&buf, b"$-1\r\n");
        buf.clear();

        // set key-value
        let set_input = b"*3\r\n$3\r\nSET\r\n$3\r\nhey\r\n$3\r\nyou\r\n";
        socket.write_all(set_input).await.unwrap();
        socket.read_buf(&mut buf).await.unwrap();
        assert_eq!(&buf, b"+OK\r\n");
        buf.clear();

        // get existing key
        let get_input = b"*2\r\n$3\r\nGET\r\n$3\r\nhey\r\n";
        socket.write_all(get_input).await.unwrap();
        socket.read_buf(&mut buf).await.unwrap();
        assert_eq!(&buf, b"$3\r\nyou\r\n");
        buf.clear();

        server_handler.abort();
        Ok(())
    }
}
