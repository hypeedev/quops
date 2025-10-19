use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum WriteError {
    ValueTooLarge(String),
}

impl Display for WriteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::ValueTooLarge(message) => write!(f, "Value too large: {}", message),
        }
    }
}

impl std::error::Error for WriteError {}

#[derive(Debug)]
pub enum ReadError {
    NotEnoughBits(String),
    InvalidBitCount(String),
}

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::NotEnoughBits(message) => write!(f, "Not enough bits: {}", message),
            ReadError::InvalidBitCount(message) => write!(f, "Invalid bit count: {}", message),
        }
    }
}

impl std::error::Error for ReadError {}

pub struct BitWriter {
    bytes: Vec<u8>,
    buffer: u64,
    buffer_filled: u8,
    bytes_written: usize,
}

impl BitWriter {
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        BitWriter {
            bytes: Vec::with_capacity(capacity),
            buffer: 0,
            buffer_filled: 0,
            bytes_written: 0,
        }
    }

    #[inline(always)]
    pub fn write(&mut self, value: u64, count: u8) -> Result<(), WriteError> {
        if count > 64 || (count < 64 && value >= (1u64 << count)) {
            return Err(WriteError::ValueTooLarge(format!(
                "Value {} exceeds the maximum for {} bits",
                value, count
            )));
        }

        if self.buffer_filled + count > 64 {
            let available_space = 64 - self.buffer_filled;
            if available_space > 0 {
                let mask = (1u64 << available_space) - 1;
                self.buffer |= (value & mask) << self.buffer_filled;
            }

            self.bytes.reserve_exact(8);

            unsafe { self.bytes.set_len(self.bytes_written + 8); }
            let ptr = unsafe { self.bytes.as_mut_ptr().add(self.bytes_written) };
            let m128i = unsafe { std::mem::transmute::<u128, std::arch::x86_64::__m128i>(self.buffer as u128) };
            unsafe { std::arch::x86_64::_mm_storeu_si64(ptr as *mut _, m128i); }
            self.bytes_written += 8;

            self.buffer = value >> available_space;
            self.buffer_filled = count - available_space;
        } else {
            self.buffer |= value << self.buffer_filled;
            self.buffer_filled += count;
        }

        Ok(())
    }

    // TODO: `self.bytes` after calling `set_len` could be less than a multiple of 8 bytes needed for `_mm_storeu_si64`,
    // TODO: resulting in writing to potentially uninitialized memory.
    #[inline(always)]
    pub fn into_bytes(mut self) -> Vec<u8> {
        let additional_bytes = ((self.buffer_filled + 7) / 8) as usize;
        let total_bytes = self.bytes_written + additional_bytes;
        self.bytes.reserve_exact(additional_bytes);
        unsafe {
            self.bytes.set_len(total_bytes);
            let ptr = self.bytes.as_mut_ptr().add(self.bytes_written);
            let m128i = std::mem::transmute::<u128, std::arch::x86_64::__m128i>(self.buffer as u128);
            std::arch::x86_64::_mm_storeu_si64(ptr as *mut _, m128i);
        }
        self.bytes
    }

    // #[inline(always)]
    // pub fn into_bytes(mut self) -> Bytes {
    //     let total_bytes = self.bytes_written + ((self.buffer_filled + 7) / 8) as usize;
    //     self.bytes.resize(total_bytes, 0);
    //     let mut ptr = unsafe { self.bytes.as_mut_ptr().add(self.bytes_written) };
    //     let mut buffer = self.buffer;
    //     let mut filled = self.buffer_filled;
    //     while filled >= 8 {
    //         unsafe {
    //             *ptr = buffer as u8;
    //             ptr = ptr.add(1);
    //         }
    //         buffer >>= 8;
    //         filled -= 8;
    //     }
    //     if filled > 0 {
    //         unsafe {
    //             *ptr = buffer as u8;
    //         }
    //     }
    //     self.bytes
    // }

    // #[inline(always)]
    // pub fn into_bytes(mut self) -> Bytes {
    //     if self.buffer_filled > 0 {
    //         match self.buffer_filled.next_power_of_two() {
    //             1 | 2 | 4 | 8 => self.bytes.push(self.buffer as u8),
    //             16 => {
    //                 // self.bytes.reserve(2);
    //                 unsafe { self.bytes.set_len(self.bytes_written + 2); }
    //                 let ptr = unsafe { self.bytes.as_mut_ptr().add(self.bytes_written) };
    //                 let m128i = unsafe { std::mem::transmute::<u128, std::arch::x86_64::__m128i>(self.buffer as u128) };
    //                 unsafe { std::arch::x86_64::_mm_storeu_si16(ptr as *mut _, m128i); }
    //             },
    //             32 => {
    //                 // self.bytes.reserve(4);
    //                 unsafe { self.bytes.set_len(self.bytes_written + 4); }
    //                 let ptr = unsafe { self.bytes.as_mut_ptr().add(self.bytes_written) };
    //                 let m128i = unsafe { std::mem::transmute::<u128, std::arch::x86_64::__m128i>(self.buffer as u128) };
    //                 unsafe { std::arch::x86_64::_mm_storeu_si32(ptr as *mut _, m128i); }
    //             },
    //             64 => {
    //                 // self.bytes.reserve(8);
    //                 unsafe { self.bytes.set_len(self.bytes_written + 8); }
    //                 let ptr = unsafe { self.bytes.as_mut_ptr().add(self.bytes_written) };
    //                 let m128i = unsafe { std::mem::transmute::<u128, std::arch::x86_64::__m128i>(self.buffer as u128) };
    //                 unsafe { std::arch::x86_64::_mm_storeu_si64(ptr as *mut _, m128i); }
    //             },
    //             _ => unreachable!()
    //         }
    //     }
    //     self.bytes
    // }

    // #[inline(always)]
    // pub fn into_bytes(mut self) -> Bytes {
    //     while self.buffer_filled >= 8 {
    //         self.bytes.push((self.buffer & 0xFF) as u8);
    //         self.buffer >>= 8;
    //         self.buffer_filled -= 8;
    //     }
    //     if self.buffer_filled > 0 {
    //         self.bytes.push((self.buffer & 0xFF) as u8);
    //     }
    //     self.bytes
    // }
}

impl Debug for BitWriter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut binary_string = String::new();
        for byte in self.bytes.as_slice().iter() {
            binary_string.push_str(&format!("{:08b}, ", byte));
        }
        binary_string.pop();
        binary_string.pop();
        write!(f, "BitWriter {{\n\tbytes: {:?},\n\tbits: [{}]\n}}", self.bytes.to_vec(), binary_string)
    }
}

pub struct BitReader<'a> {
    bytes: &'a [u8],
    bits: usize,
    bit_position: usize,
    buffer: u128,
    filled: u8,
    byte_idx: usize,
}

impl<'a> BitReader<'a> {
    #[inline(always)]
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReader {
            bytes,
            bits: bytes.len() * 8,
            bit_position: 0,
            buffer: 0,
            filled: 0,
            byte_idx: 0,
        }
    }

    // #[inline(always)]
    // pub fn read(&mut self, mut count: u8) -> Result<u64, ReadError> {
    //     if count > 64 {
    //         return Err(ReadError::InvalidBitCount(format!("Requested {} bits, but maximum is 64", count)));
    //     }
    //
    //     let mut pos = self.bit_position;
    //
    //     let available_bits = self.bits - pos;
    //     if (count as usize) > available_bits {
    //         return Err(ReadError::NotEnoughBits(format!("Requested {} bits, but only {} bits available", count, available_bits)));
    //     }
    //
    //     let mut value: u64 = 0;
    //     let mut shift: u32 = 0;
    //
    //     while count > 0 {
    //         let byte_idx = pos >> 3;
    //         let bit_offset = pos & 7;
    //
    //         let bits_in_this_byte = (8 - bit_offset).min(count as usize) as u8;
    //
    //         let byte = unsafe { *self.bytes.get_unchecked(byte_idx) };
    //         let mask = ((1u16 << bits_in_this_byte) - 1) as u8;
    //
    //         let bits = (byte >> bit_offset) & mask;
    //         value |= (bits as u64) << shift;
    //
    //         shift += bits_in_this_byte as u32;
    //         pos += bits_in_this_byte as usize;
    //         count -= bits_in_this_byte;
    //     }
    //
    //     self.bit_position = pos;
    //     Ok(value)
    // }

    // #[inline(always)]
    // pub fn read(&mut self, count: u8) -> Result<u64, ReadError> {
    //     // TODO: Double check if we can get rid of these 2 checks. All `count` values are known at compile time.
    //
    //     if count > 64 {
    //         return Err(ReadError::InvalidBitCount(format!("Requested {} bits, but maximum is 64", count)));
    //     }
    //
    //     let available_bits = self.bits - self.bit_position;
    //     if (count as usize) > available_bits {
    //         return Err(ReadError::NotEnoughBits(format!("Requested {} bits, but only {} bits available", count, available_bits)));
    //     }
    //
    //     if self.filled < count {
    //         unsafe {
    //             let ptr = self.bytes.as_ptr().add(self.byte_idx);
    //             let value = std::arch::x86_64::_mm_loadu_si64(ptr as *const _);
    //             let value: [u64; 2] = std::mem::transmute(value);
    //             self.buffer |= (value[0] as u128) << self.filled;
    //             self.filled += 64;
    //             self.byte_idx += 8;
    //         }
    //     }
    //
    //     let mask = (((1u64 << (count - 1)) - 1) << 1) + 1;
    //     let value = (self.buffer & mask as u128) as u64;
    //     self.buffer >>= count as u128;
    //     self.filled -= count;
    //
    //     self.bit_position += count as usize;
    //
    //     Ok(value)
    // }

    #[inline(always)]
    pub fn read(&mut self, count: u8) -> Result<u64, ReadError> {
        // Every field in the schema has a bit count of less than or equal to 64.
        // Therefore, we can safely assume that `count` is always less than or equal to 64.

        let available_bits = self.bits - self.bit_position;
        if (count as usize) > available_bits {
            return Err(ReadError::NotEnoughBits(format!("Requested {} bits, but only {} bits available", count, available_bits)));
        }

        if self.filled < count {
            unsafe {
                let ptr = self.bytes.as_ptr().add(self.byte_idx);
                let value = std::arch::x86_64::_mm_loadu_si64(ptr as *const _);
                let value: [u64; 2] = std::mem::transmute(value);
                self.buffer |= (value[0] as u128) << self.filled;
                self.filled += 64;
                self.byte_idx += 8;
            }
        }

        let mask = (((1u64 << (count - 1)) - 1) << 1) + 1;
        let value = (self.buffer & mask as u128) as u64;
        self.buffer >>= count as u128;
        self.filled -= count;

        self.bit_position += count as usize;

        Ok(value)
    }
}

impl<'a> Debug for BitReader<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut binary_string = String::new();
        for byte in self.bytes {
            binary_string.push_str(&format!("{:08b}, ", byte));
        }
        binary_string.pop();
        binary_string.pop();
        write!(f, "BitReader {{\n\tbytes: {:?},\n\tbits: [{}],\n\tbit_position: {}\n}}", self.bytes, binary_string, self.bit_position)
    }
}
