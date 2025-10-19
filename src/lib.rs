// TODO: Add documentation and examples
// TODO: Add tests for encoding and decoding
// TODO: Add support for unsigned, unbounded integers
// TODO: Add support for `min` without `max` or `max` without `min`
// TODO: Add support for string fields
// TODO: Write .quops schema definition ($schema)

pub mod bit;
pub mod traits;
mod errors;

pub use bit::{BitReader, BitWriter};
pub use quops_derive::{Decode, Encode};
pub use errors::{DecodeError, EncodeError};

#[inline(always)]
pub fn encode<T: traits::Encode>(value: &T) -> Result<Vec<u8>, EncodeError> {
    traits::Encode::encode(value)
}

#[inline(always)]
pub fn decode<T: traits::Decode>(buffer: &[u8]) -> Result<T, DecodeError> {
    traits::Decode::decode(buffer)
}
