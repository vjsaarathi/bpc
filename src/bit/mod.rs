//! Bit-level reading and writing engine.
//!
//! Provides [`BitReader`] and [`BitWriter`] for working with arbitrary-width
//! bit fields using MSB-first (big-endian) ordering.
//!
//! # Bit ordering
//!
//! All operations use MSB-first bit ordering: the most significant bit of each
//! byte is read/written first. Multi-byte values use big-endian byte order.
//!
//! # Examples
//!
//! ```
//! use bpc::bit::{BitReader, BitWriter};
//!
//! // Write some fields.
//! let mut writer = BitWriter::new();
//! writer.write_bits(0b101, 3).unwrap();   // 3-bit field
//! writer.write_bits(0b10110, 5).unwrap(); // 5-bit field
//! writer.write_u16(0x1234, 16).unwrap();  // 16-bit field
//!
//! // Read them back.
//! let bit_len = writer.bit_len();
//! let bytes = writer.into_bytes();
//! let mut reader = BitReader::new(&bytes, bit_len);
//!
//! assert_eq!(reader.read_bits(3).unwrap(), 0b101);
//! assert_eq!(reader.read_bits(5).unwrap(), 0b10110);
//! assert_eq!(reader.read_u16(16).unwrap(), 0x1234);
//! ```

pub mod error;
pub mod reader;
pub mod writer;

pub use error::{BitError, BitResult};
pub use reader::BitReader;
pub use writer::BitWriter;
