use anyhow::Result;

use crate::redis::{Command, Frame, Storage};
use crate::Connection;

pub(crate) struct ConnectionHandler {
    connection: Connection,
    storage: Storage,
}

impl ConnectionHandler {
    pub fn new(connection: Connection, storage: Storage) -> Self {
        ConnectionHandler {
            connection: connection,
            storage: storage,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            let buffer = self.connection.read().await?;
            let frame = Frame::from_bytes(&buffer)?;
            let mut cmd: Command = Command::from_frame(&frame)?;
            match &mut cmd {
                Command::Get(cmd) => cmd.run(&self.storage).await,
                Command::Set(cmd) => cmd.run(&self.storage).await,
                _ => {}
            };

            let response_frame = cmd.as_response_frame();
            self.connection
                .write(&response_frame.as_resp_bytes())
                .await?;
        }
    }
}
