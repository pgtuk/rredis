use anyhow::Result;
use bytes::Bytes;

use crate::redis::command::{CommandArgs, RESPCommand};
use crate::redis::Frame;

#[derive(Debug)]
pub(crate) struct Echo {
    message: Bytes,
}

impl RESPCommand for Echo {
    const NAME: &'static str = "echo";

    fn parse(args: &mut CommandArgs) -> Result<Echo> {
        let message = args.next_bytes()?;
        Ok(Echo { message })
    }

    fn to_response(&self) -> Frame {
        Frame::BulkString(self.message.clone())
    }
}
