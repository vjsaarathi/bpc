//! Bit-level writer for constructing arbitrary-width fields into a byte buffer.
//!
//! Bits are written in MSB-first (big-endian) order. Multi-byte values use
//! big-endian byte order.

use super::error::{BitError, BitResult};

/// A writer that packs arbitrary-width bit fields into an owned byte buffer.
///
/// Bits are written MSB-first (most significant bit first) within each byte.
/// Multi-byte values are written in big-endian byte order.
///
/// # Examples
///
/// ```
/// use bpc::bit::BitWriter;
///
/// let mut writer = BitWriter::new();
/// writer.write_bits(0b1101, 4).unwrap();
/// writer.write_bits(0b0011, 4).unwrap();
/// assert_eq!(writer.into_bytes(), vec![0b1101_0011]);
/// ```
#[derive(Debug, Clone)]
pub struct BitWriter {
    buffer: Vec<u8>,
    bit_len: usize,
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BitWriter {
    /// Creates a new empty `BitWriter`.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            bit_len: 0,
        }
    }

    /// Creates a new `BitWriter` with pre-allocated capacity for at least
    /// `bit_capacity` bits.
    pub fn with_capacity(bit_capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity((bit_capacity + 7) / 8),
            bit_len: 0,
        }
    }

    /// Returns the current bit position (number of bits written so far).
    pub fn position(&self) -> usize {
        self.bit_len
    }

    /// Returns the total number of bits written.
    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// Writes a single bit (`true` = 1, `false` = 0).
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::bit::BitWriter;
    ///
    /// let mut writer = BitWriter::new();
    /// writer.write_bit(true);
    /// writer.write_bit(false);
    /// writer.write_bit(true);
    /// writer.align_to_byte();
    /// assert_eq!(writer.into_bytes(), vec![0b1010_0000]);
    /// ```
    pub fn write_bit(&mut self, bit: bool) {
        let byte_idx = self.bit_len / 8;
        let bit_offset = self.bit_len % 8;

        if byte_idx >= self.buffer.len() {
            self.buffer.push(0);
        }

        if bit {
            self.buffer[byte_idx] |= 1 << (7 - bit_offset);
        }

        self.bit_len += 1;
    }

    /// Writes the lowest `n` bits of `value` in MSB-first order.
    ///
    /// Writing 0 bits is a no-op.
    ///
    /// # Errors
    ///
    /// - [`BitError::InvalidBitCount`] if `n > 64`.
    /// - [`BitError::ValueOverflow`] if `value` does not fit in `n` bits.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::bit::BitWriter;
    ///
    /// let mut writer = BitWriter::new();
    /// writer.write_bits(0b101, 3).unwrap();
    /// writer.write_bits(0b10, 2).unwrap();
    /// writer.align_to_byte();
    /// assert_eq!(writer.into_bytes(), vec![0b10110_000]);
    /// ```
    pub fn write_bits(&mut self, value: u64, n: u32) -> BitResult<()> {
        if n == 0 {
            return Ok(());
        }
        if n > 64 {
            return Err(BitError::InvalidBitCount { bits: n, max: 64 });
        }
        // Check that value fits in n bits.
        if n < 64 && (value >> n) != 0 {
            return Err(BitError::ValueOverflow {
                value: u128::from(value),
                bits: n,
            });
        }

        let n_bits = n as usize;
        let mut bits_written = 0;

        // Process bits in byte-aligned chunks for efficiency.
        while bits_written < n_bits {
            let byte_idx = self.bit_len / 8;
            let bit_offset = self.bit_len % 8;
            let space = 8 - bit_offset;
            let to_write = space.min(n_bits - bits_written);

            if byte_idx >= self.buffer.len() {
                self.buffer.push(0);
            }

            // Extract `to_write` bits from value starting at position
            // `bits_written` (MSB-first).
            let src_shift = n_bits - bits_written - to_write;
            let mask = (1u64 << to_write) - 1;
            let bits = ((value >> src_shift) & mask) as u8;

            // Place into byte at the correct position.
            let dst_shift = space - to_write;
            self.buffer[byte_idx] |= bits << dst_shift;

            self.bit_len += to_write;
            bits_written += to_write;
        }

        Ok(())
    }

    /// Writes a `u8` value using `n` bits.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 8`.
    /// Returns [`BitError::ValueOverflow`] if `value` does not fit in `n` bits.
    pub fn write_u8(&mut self, value: u8, n: u32) -> BitResult<()> {
        if n > 8 {
            return Err(BitError::InvalidBitCount { bits: n, max: 8 });
        }
        self.write_bits(u64::from(value), n)
    }

    /// Writes a `u16` value using `n` bits.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 16`.
    /// Returns [`BitError::ValueOverflow`] if `value` does not fit in `n` bits.
    pub fn write_u16(&mut self, value: u16, n: u32) -> BitResult<()> {
        if n > 16 {
            return Err(BitError::InvalidBitCount { bits: n, max: 16 });
        }
        self.write_bits(u64::from(value), n)
    }

    /// Writes a `u32` value using `n` bits.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 32`.
    /// Returns [`BitError::ValueOverflow`] if `value` does not fit in `n` bits.
    pub fn write_u32(&mut self, value: u32, n: u32) -> BitResult<()> {
        if n > 32 {
            return Err(BitError::InvalidBitCount { bits: n, max: 32 });
        }
        self.write_bits(u64::from(value), n)
    }

    /// Writes a `u64` value using `n` bits. Equivalent to [`write_bits`](Self::write_bits).
    pub fn write_u64(&mut self, value: u64, n: u32) -> BitResult<()> {
        self.write_bits(value, n)
    }

    /// Aligns to the next byte boundary by padding with zero bits.
    ///
    /// If already byte-aligned, this is a no-op.
    pub fn align_to_byte(&mut self) {
        let remainder = self.bit_len % 8;
        if remainder != 0 {
            // The current partial byte was initialized to 0, so the padding
            // bits are already zero. Just advance the position.
            self.bit_len += 8 - remainder;
        }
    }

    /// Consumes the writer and returns the accumulated bytes.
    ///
    /// If the last byte is only partially written, it is included with
    /// unwritten bits set to 0.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Returns a reference to the accumulated bytes so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_writer_is_empty() {
        let writer = BitWriter::new();
        assert_eq!(writer.position(), 0);
        assert_eq!(writer.bit_len(), 0);
        assert!(writer.as_bytes().is_empty());
    }

    #[test]
    fn write_single_bit() {
        let mut writer = BitWriter::new();
        writer.write_bit(true);
        writer.align_to_byte();
        assert_eq!(writer.into_bytes(), vec![0b1000_0000]);
    }

    #[test]
    fn write_all_bits_one_by_one() {
        let mut writer = BitWriter::new();
        // Write 1010_0110
        for bit in [true, false, true, false, false, true, true, false] {
            writer.write_bit(bit);
        }
        assert_eq!(writer.into_bytes(), vec![0b1010_0110]);
    }

    #[test]
    fn write_bits_within_byte() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b1101, 4).unwrap();
        writer.write_bits(0b0011, 4).unwrap();
        assert_eq!(writer.into_bytes(), vec![0b1101_0011]);
    }

    #[test]
    fn write_bits_crossing_boundary() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b10110, 5).unwrap();
        writer.write_bits(0b0110100, 7).unwrap();
        writer.align_to_byte();
        assert_eq!(writer.into_bytes(), vec![0b1011_0011, 0b0100_0000]);
    }

    #[test]
    fn write_full_u64() {
        let mut writer = BitWriter::new();
        writer.write_u64(0x0123456789ABCDEF, 64).unwrap();
        assert_eq!(
            writer.into_bytes(),
            vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]
        );
    }

    #[test]
    fn write_zero_bits_is_noop() {
        let mut writer = BitWriter::new();
        writer.write_bits(42, 0).unwrap();
        assert_eq!(writer.position(), 0);
    }

    #[test]
    fn write_value_overflow() {
        let mut writer = BitWriter::new();
        let err = writer.write_bits(0xFF, 7).unwrap_err();
        assert_eq!(
            err,
            BitError::ValueOverflow {
                value: 0xFF,
                bits: 7,
            }
        );
    }

    #[test]
    fn write_max_value_fits() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b1111111, 7).unwrap();
        assert_eq!(writer.position(), 7);
    }

    #[test]
    fn write_invalid_bit_count() {
        let mut writer = BitWriter::new();
        let err = writer.write_bits(0, 65).unwrap_err();
        assert_eq!(err, BitError::InvalidBitCount { bits: 65, max: 64 });
    }

    #[test]
    fn write_u8_too_wide() {
        let mut writer = BitWriter::new();
        let err = writer.write_u8(0, 9).unwrap_err();
        assert_eq!(err, BitError::InvalidBitCount { bits: 9, max: 8 });
    }

    #[test]
    fn write_u8_overflow() {
        let mut writer = BitWriter::new();
        let err = writer.write_u8(255, 7).unwrap_err();
        assert_eq!(
            err,
            BitError::ValueOverflow {
                value: 255,
                bits: 7,
            }
        );
    }

    #[test]
    fn align_pads_with_zeros() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b111, 3).unwrap();
        writer.align_to_byte();
        assert_eq!(writer.position(), 8);
        assert_eq!(writer.into_bytes(), vec![0b1110_0000]);
    }

    #[test]
    fn align_already_aligned() {
        let mut writer = BitWriter::new();
        writer.write_bits(0xFF, 8).unwrap();
        writer.align_to_byte();
        assert_eq!(writer.position(), 8);
    }

    #[test]
    fn align_empty_is_noop() {
        let mut writer = BitWriter::new();
        writer.align_to_byte();
        assert_eq!(writer.position(), 0);
    }

    #[test]
    fn position_tracks() {
        let mut writer = BitWriter::new();
        assert_eq!(writer.position(), 0);
        writer.write_bits(0b11, 2).unwrap();
        assert_eq!(writer.position(), 2);
        writer.write_bit(true);
        assert_eq!(writer.position(), 3);
        writer.align_to_byte();
        assert_eq!(writer.position(), 8);
    }

    #[test]
    fn as_bytes_reflects_state() {
        let mut writer = BitWriter::new();
        writer.write_bits(0xAB, 8).unwrap();
        assert_eq!(writer.as_bytes(), &[0xAB]);
        writer.write_bits(0xCD, 8).unwrap();
        assert_eq!(writer.as_bytes(), &[0xAB, 0xCD]);
    }

    #[test]
    fn with_capacity_works() {
        let mut writer = BitWriter::with_capacity(64);
        writer.write_bits(0xFF, 8).unwrap();
        assert_eq!(writer.into_bytes(), vec![0xFF]);
    }

    #[test]
    fn default_is_new() {
        let writer: BitWriter = Default::default();
        assert_eq!(writer.position(), 0);
    }
}
