use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::num::TryFromIntError;
use crate::bit::{ReadError, WriteError};

#[derive(Debug)]
pub enum EncodeError {
    OutOfBounds(String),
    NotSupported(String),
}

impl Display for EncodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::OutOfBounds(msg) => write!(f, "Encoding error: Out of bounds - {}", msg),
            EncodeError::NotSupported(msg) => write!(f, "Encoding error: Not supported - {}", msg),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<WriteError> for EncodeError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::ValueTooLarge(msg) => EncodeError::OutOfBounds(msg),
        }
    }
}

#[derive(Debug)]
pub enum DecodeError {
    OutOfBounds(String),
    NotEnoughBytes(String),
    NotEnoughBits(String),
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::OutOfBounds(msg) => write!(f, "Decoding error: Out of bounds - {}", msg),
            DecodeError::NotEnoughBytes(msg) => write!(f, "Decoding error: Not enough bytes - {}", msg),
            DecodeError::NotEnoughBits(msg) => write!(f, "Decoding error: Not enough bits - {}", msg),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<ReadError> for DecodeError {
    fn from(error: ReadError) -> Self {
        match error {
            ReadError::NotEnoughBits(msg) => DecodeError::NotEnoughBits(msg),
            ReadError::InvalidBitCount(msg) => DecodeError::OutOfBounds(msg),
        }
    }
}

impl From<TryFromIntError> for DecodeError {
    fn from(error: TryFromIntError) -> Self {
        DecodeError::OutOfBounds(error.to_string())
    }
}

impl From<Infallible> for DecodeError {
    fn from(_error: Infallible) -> Self {
        DecodeError::OutOfBounds("Infallible error occurred".to_string())
    }
}