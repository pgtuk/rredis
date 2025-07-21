use anyhow::Result;
use bytes::Bytes;

use super::{CommandArgs, RESPCommand};
use crate::redis::{Frame, Storage};

#[derive(Debug)]
pub(crate) struct Get {
    key: String,
    result: Option<Bytes>,
}

impl Get {
    pub(crate) async fn run(&mut self, storage: &Storage) {
        self.result = storage.get(&self.key).await;
    }
}

impl RESPCommand for Get {
    const NAME: &'static str = "get";

    fn parse(args: &mut CommandArgs) -> Result<Get> {
        let key = args.next_bytes()?;
        Ok(Get {
            key: String::from_utf8(key.to_vec())?,
            result: None,
        })
    }

    fn to_response(&self) -> Frame {
        match &self.result {
            Some(value) => Frame::BulkString(value.clone()),
            None => Frame::Null,
        }
    }
}
