use crate::errors::{EncodeError, DecodeError};

pub trait Encode {
    fn encode(&self) -> Result<Vec<u8>, EncodeError>;
}

pub trait Decode: Sized {
    fn decode(bytes: &[u8]) -> Result<Self, DecodeError>;
}

pub trait AsU64 {
    fn as_u64(&self) -> Result<u64, EncodeError>;
}
