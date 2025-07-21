use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
pub(crate) struct Connection {
    stream: TcpStream,
    buffer_size: usize,
}

impl Connection {
    pub fn new(stream: TcpStream, buffer_size: usize) -> Connection {
        Connection {
            stream: stream,
            buffer_size: buffer_size, // BytesMut::with_capacity(buffer_size),
        }
    }

    pub async fn read(&mut self) -> Result<Vec<u8>> {
        // TODO: handle closed conn
        loop {
            let mut buffer = Vec::with_capacity(self.buffer_size);
            self.stream.read_buf(&mut buffer).await?;

            if !buffer.is_empty() {
                return Ok(buffer);
            }
        }
    }

    pub async fn write(&mut self, response: &[u8]) -> Result<()> {
        self.stream.write(response).await?;
        Ok(())
    }
}
