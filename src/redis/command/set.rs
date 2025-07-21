use anyhow::Result;
use bytes::Bytes;

use super::{CommandArgs, RESPCommand};
use crate::redis::{Frame, Storage};

#[derive(Debug)]
pub(crate) struct Set {
    key: String,
    value: Bytes,
}

impl Set {
    pub(crate) async fn run(&self, storage: &Storage) {
        storage.set(&self.key, &self.value).await;
    }
}

impl RESPCommand for Set {
    const NAME: &'static str = "set";

    fn parse(args: &mut CommandArgs) -> Result<Set> {
        let key = args.next_bytes()?;
        let value = args.next_bytes()?;

        Ok(Set {
            key: String::from_utf8(key.to_vec())?,
            value,
        })
    }

    fn to_response(&self) -> Frame {
        Frame::SimpleString(Bytes::from_static(b"OK"))
    }
}
