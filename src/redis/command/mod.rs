use anyhow::Result;
use bytes::Bytes;
use std::slice::Iter;

use crate::redis::CmdErrors;
use crate::redis::Frame;

mod ping;
use ping::Ping;
mod echo;
use echo::Echo;
mod set;
use set::Set;
mod get;
use get::Get;

pub(crate) struct CommandArgs<'a>(Iter<'a, Frame>);

impl CommandArgs<'_> {
    pub fn next_bytes(&mut self) -> Result<Bytes> {
        match self.0.next() {
            Some(Frame::BulkString(value)) => Ok(value.clone()),
            Some(wrong_frame) => Err(CmdErrors::IncorrectCommandArg {
                command_name: Get::NAME,
                arg: format!("{}", wrong_frame),
            }
            .into()),
            None => Err(CmdErrors::MissingCommandArg {
                command_name: Get::NAME,
                // TODO
                arg_name: "value",
            }
            .into()),
        }
    }
}

pub(crate) trait RESPCommand: Sized {
    const NAME: &'static str;
    fn parse(args: &mut CommandArgs) -> Result<Self>;
    fn to_response(&self) -> Frame;
}

// / Note-to-self: using additional `enum Command` over just using structs that implement `trait RESPCommand`
// / avoids dynamic dispatch
#[derive(Debug)]
pub(crate) enum Command {
    Ping(Ping),
    Echo(Echo),
    Set(Set),
    Get(Get),
}

impl Command {
    pub fn from_frame(frame: &Frame) -> Result<Command> {
        let parts = Command::validate(&frame)?;
        let mut args = CommandArgs {
            0: parts[1..].iter(),
        };
        let cmd = match &parts[0].as_string()?.to_lowercase()[..] {
            Ping::NAME => Command::Ping(Ping::parse(&mut args)?),
            Echo::NAME => Command::Echo(Echo::parse(&mut args)?),
            Set::NAME => Command::Set(Set::parse(&mut args)?),
            Get::NAME => Command::Get(Get::parse(&mut args)?),
            unknown => {
                return Err(CmdErrors::UnknownCommand(unknown.to_owned()).into());
            }
        };

        Ok(cmd)
    }

    pub fn as_response_frame(&self) -> Frame {
        match self {
            Command::Ping(ping) => ping.to_response(),
            Command::Echo(echo) => echo.to_response(),
            Command::Set(set) => set.to_response(),
            Command::Get(get) => get.to_response(),
        }
    }

    fn validate(frame: &Frame) -> Result<&Vec<Frame>, CmdErrors> {
        match frame {
            Frame::Array(frames) if frames.len() > 0 => Ok(frames),
            _ => Err(CmdErrors::InvalidArrayFrame),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_validate() {
        let frame = Frame::Array(vec![
            Frame::BulkString(Bytes::from("hello")),
            Frame::BulkString(Bytes::from("world")),
        ]);

        assert_eq!(
            Command::validate(&frame).unwrap(),
            &vec![
                Frame::BulkString(Bytes::from("hello")),
                Frame::BulkString(Bytes::from("world")),
            ]
        )
    }
}
