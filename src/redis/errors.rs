use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub(crate) enum FrameErrors {
    #[error("unexpected first byte: 0x{0:02X}")]
    IncorrectFirstByte(u8),

    #[error("missing CRLF terminator")]
    MissingCRLF,

    #[error("Incorrect bulk string length")]
    IncorrectBulkStringLength,

    #[error("Array items must be bulk strings")]
    WrongArrayItemFormat,

    #[error("")]
    StringInterpretationError,
}

#[derive(Debug, Error, PartialEq)]
pub(crate) enum CmdErrors {
    #[error("Frame is not an array frame or empty")]
    InvalidArrayFrame,

    #[error("Wrong arg - {arg:}, for {command_name:}")]
    IncorrectCommandArg {
        command_name: &'static str,
        arg: String,
    },

    #[error("Wrong or missing args for {command_name:}, args - {arg_name:}")]
    MissingCommandArg {
        command_name: &'static str,
        arg_name: &'static str,
    },

    #[error("`{0}`")]
    UnknownCommand(String),
}
