use anyhow::Result;
use bytes::Bytes;

use super::{CommandArgs, RESPCommand};
use crate::redis::Frame;

#[derive(Debug)]
pub(crate) struct Ping {}

impl RESPCommand for Ping {
    const NAME: &'static str = "ping";

    fn parse(_: &mut CommandArgs) -> Result<Ping> {
        Ok(Ping {})
    }

    fn to_response(&self) -> Frame {
        Frame::SimpleString(Bytes::from_static(b"PONG"))
    }
}
