// RESP encoding/decoding modul
use bytes::Bytes;
use std::{fmt::Display, unimplemented};

use crate::redis::FrameErrors;

#[repr(u8)]
enum SpecialBytes {
    CR = b'\r',
    LF = b'\n',
}

#[repr(u8)]
enum FirstByte {
    Plus = b'+',   // simple string
    Colon = b':',  // integer
    Dollar = b'$', // bulk string
    Star = b'*',   // array
}

impl TryFrom<u8> for FirstByte {
    type Error = FrameErrors;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            value if value == FirstByte::Plus as u8 => Ok(FirstByte::Plus),
            value if value == FirstByte::Star as u8 => Ok(FirstByte::Star),
            value if value == FirstByte::Dollar as u8 => Ok(FirstByte::Dollar),
            value if value == FirstByte::Colon as u8 => Ok(FirstByte::Colon),
            other => Err(FrameErrors::IncorrectFirstByte(other)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Frame {
    Array(Vec<Frame>),
    SimpleString(Bytes),
    BulkString(Bytes),
    Integer(i64),
    Null,
}

impl Frame {
    //// RESP serialized
    pub fn as_resp_bytes(&self) -> Vec<u8> {
        match self {
            Frame::SimpleString(val) => encode_simple_string(val),
            Frame::BulkString(val) => encode_bulk_string(val),
            Frame::Null => encode_null(),
            _ => unimplemented!(),
        }
    }

    pub fn from_bytes(buffer: &[u8]) -> Result<Frame, FrameErrors> {
        Frame::check(buffer)?;
        let frame = match FirstByte::try_from(buffer[0])? {
            FirstByte::Plus => Frame::SimpleString(decode_simple_string(&buffer[1..])?),
            FirstByte::Colon => Frame::Integer(decode_integer(&buffer[1..])?),
            FirstByte::Star => Frame::Array(decode_array(&buffer[1..])?),
            FirstByte::Dollar => Frame::BulkString(decode_bulk_string(&buffer[1..])?),
        };

        Ok(frame)
    }

    pub fn as_string(&self) -> Result<String, FrameErrors> {
        match self {
            Frame::SimpleString(val) | Frame::BulkString(val) => Ok(std::str::from_utf8(&val)
                .map_err(|_| FrameErrors::StringInterpretationError)?
                .to_string()),
            _ => Err(FrameErrors::StringInterpretationError),
        }
    }

    fn check(buffer: &[u8]) -> Result<(), FrameErrors> {
        let lenght = buffer.len();
        // command must end with `\r\n`
        if buffer[lenght - 2] != SpecialBytes::CR as u8
            || buffer[lenght - 1] != SpecialBytes::LF as u8
        {
            return Err(FrameErrors::MissingCRLF);
        }

        Ok(())
    }
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Frame::SimpleString(val) => format!("Simple string - {:?}", val),
            Frame::BulkString(val) => format!("Bulk string - {:?}", val),
            Frame::Integer(val) => format!("Integer - {}", val),
            Frame::Array(val) => format!("Array - [{:?}]", val),
            Frame::Null => "Null".to_string(),
        };

        write!(f, "{}", repr)
    }
}

fn decode_simple_string(buffer: &[u8]) -> Result<Bytes, FrameErrors> {
    let length = get_input_length(buffer)?;
    Ok(Bytes::copy_from_slice(&buffer[0..length]))
}

fn decode_bulk_string(buffer: &[u8]) -> Result<Bytes, FrameErrors> {
    let data_len = decode_integer(buffer)? as usize;
    let start = 1 + get_position(buffer, SpecialBytes::LF as u8).ok_or(FrameErrors::MissingCRLF)?;

    if buffer[start..start + data_len]
        .iter()
        .any(|&c| is_special_byte(&c))
    {
        return Err(FrameErrors::IncorrectBulkStringLength);
    }

    if buffer[start + data_len] != SpecialBytes::CR as u8 {
        return Err(FrameErrors::IncorrectBulkStringLength);
    }

    Ok(Bytes::copy_from_slice(&buffer[start..start + data_len]))
}

fn decode_integer(buffer: &[u8]) -> Result<i64, FrameErrors> {
    let offset: usize;
    let sign_multiplier: i64;

    // +/- before integer is optional in RESP
    match buffer[0] {
        b'+' => {
            offset = 1;
            sign_multiplier = 1
        }
        b'-' => {
            offset = 1;
            sign_multiplier = -1
        }
        _ => {
            offset = 0;
            sign_multiplier = 1
        }
    }

    let length = get_input_length(buffer)?;
    let mut number: i64 = 0;

    for i in offset..length {
        number = number * 10 + i64::from(buffer[i] - b'0')
    }

    return Ok(number * sign_multiplier);
}

fn decode_array(buffer: &[u8]) -> Result<Vec<Frame>, FrameErrors> {
    let items_count = decode_integer(buffer)?;
    let mut items: Vec<Frame> = Vec::new();

    // start reading array items by skipping `*<number-of-elements>\r\n` part
    let mut item_start = get_input_length(buffer)? + 2;
    for _ in 0..items_count {
        let slice = &buffer[item_start..];

        let first_lf =
            get_position(slice, SpecialBytes::LF as u8).ok_or(FrameErrors::WrongArrayItemFormat)?;

        let second_lf = get_position(&slice[first_lf + 1..], SpecialBytes::LF as u8)
            .ok_or(FrameErrors::WrongArrayItemFormat)?;

        // +1 for each LF
        let item_length = first_lf + 1 + second_lf + 1;

        match Frame::from_bytes(&slice[..item_length])? {
            Frame::BulkString(val) => {
                item_start += item_length;
                items.push(Frame::BulkString(val));
            }
            _ => {
                return Err(FrameErrors::WrongArrayItemFormat);
            }
        }
    }

    Ok(items)
}

fn encode_simple_string(val: &Bytes) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(3 + val.len());

    buffer.push(FirstByte::Plus as u8);
    buffer.extend_from_slice(val);
    buffer.push(SpecialBytes::CR as u8);
    buffer.push(SpecialBytes::LF as u8);

    buffer
}

fn encode_bulk_string(val: &Bytes) -> Vec<u8> {
    let len_str = val.len().to_string();
    let mut buffer = Vec::with_capacity(5 + len_str.len() + val.len());

    buffer.push(FirstByte::Dollar as u8);
    buffer.extend_from_slice(len_str.as_bytes());
    buffer.push(SpecialBytes::CR as u8);
    buffer.push(SpecialBytes::LF as u8);
    buffer.extend_from_slice(val);
    buffer.push(SpecialBytes::CR as u8);
    buffer.push(SpecialBytes::LF as u8);

    buffer
}

fn encode_null() -> Vec<u8> {
    b"$-1\r\n".to_vec()
}

fn get_input_length(buffer: &[u8]) -> Result<usize, FrameErrors> {
    // number of bytes before first `\r`
    get_position(buffer, SpecialBytes::CR as u8).ok_or(FrameErrors::MissingCRLF)
}

fn get_position(slice: &[u8], char_we_look_for: u8) -> Option<usize> {
    slice.iter().position(|&c| c == char_we_look_for)
}

fn is_special_byte(b: &u8) -> bool {
    return *b == SpecialBytes::CR as u8 || *b == SpecialBytes::LF as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_parse_simple_string() {
        let mut buffer = BytesMut::with_capacity(64);
        buffer.extend_from_slice(b"+OK\r\n");

        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::SimpleString(Bytes::from_static(b"OK"));

        assert_eq!(expected, frame);
    }

    #[test]
    fn test_parse_integer() {
        let mut buffer = BytesMut::with_capacity(64);

        buffer.extend_from_slice(b":22\r\n");
        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::Integer(22);
        assert_eq!(expected, frame);

        buffer.clear();
        buffer.extend_from_slice(b":+423232341231233\r\n");
        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::Integer(423232341231233);
        assert_eq!(expected, frame);

        buffer.clear();
        buffer.extend_from_slice(b":-22\r\n");
        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::Integer(-22);
        assert_eq!(expected, frame);
    }

    #[test]
    fn test_parse_bulk_string() {
        let mut buffer = BytesMut::with_capacity(64);
        buffer.extend_from_slice(b"$5\r\nhello\r\n");

        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::BulkString(Bytes::from_static(b"hello"));

        assert_eq!(expected, frame);
    }

    #[test]
    fn test_parse_bulk_string_with_incorrect_length() {
        let mut buffer = BytesMut::with_capacity(64);

        buffer.extend_from_slice(b"$4\r\nhello\r\n");
        assert_eq!(
            Frame::from_bytes(&buffer).unwrap_err(),
            FrameErrors::IncorrectBulkStringLength
        );

        buffer.clear();
        buffer.extend_from_slice(b"$6\r\nhello\r\n");
        assert_eq!(
            Frame::from_bytes(&buffer).unwrap_err(),
            FrameErrors::IncorrectBulkStringLength
        );
    }

    #[test]
    fn test_parse_array() {
        let mut buffer = BytesMut::with_capacity(64);
        buffer.extend_from_slice(b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n");
        let frame = Frame::from_bytes(&buffer).unwrap();
        let expected = Frame::Array(vec![
            Frame::BulkString(Bytes::from_static(b"hello")),
            Frame::BulkString(Bytes::from_static(b"world")),
        ]);

        assert_eq!(expected, frame);
    }

    #[test]
    fn test_parse_incorrect_command() {
        let mut buffer = BytesMut::with_capacity(64);

        buffer.extend_from_slice(b":-");
        assert_eq!(
            Frame::from_bytes(&buffer).unwrap_err(),
            FrameErrors::MissingCRLF
        );

        buffer.clear();
        buffer.extend_from_slice(b"+OK\r");
        assert_eq!(
            Frame::from_bytes(&buffer).unwrap_err(),
            FrameErrors::MissingCRLF
        );

        buffer.clear();
        buffer.extend_from_slice(b"*2\r\n$5\r\nhell\r\n$5\r\nworld\r\n"); // wrong first item len
        assert_eq!(
            Frame::from_bytes(&buffer).unwrap_err(),
            FrameErrors::IncorrectBulkStringLength
        );
    }

    #[test]
    fn test_encode_simple_string() {
        let input = Bytes::from_static(b"Simple");
        let expected = b"+Simple\r\n";
        assert_eq!(encode_simple_string(&input), expected);
    }

    #[test]
    fn test_encode_null() {
        let expected = b"_\r\n";
        assert_eq!(encode_null(), expected);
    }

    #[test]
    fn test_encode_bulk_string() {
        let input = Bytes::from_static(b"hello");
        let expected = b"$5\r\nhello\r\n";
        assert_eq!(encode_bulk_string(&input), expected);
    }
}
