//! Bit-level reader for extracting arbitrary-width fields from a byte slice.
//!
//! Bits are read in MSB-first (big-endian) order: the most significant bit
//! of each byte is read first. Multi-byte values use big-endian byte order.

use super::error::{BitError, BitResult};

/// A reader that extracts arbitrary-width bit fields from a borrowed byte slice.
///
/// Bits are read MSB-first (most significant bit first) within each byte.
/// Multi-byte values are read in big-endian byte order.
///
/// # Examples
///
/// ```
/// use bpc::bit::BitReader;
///
/// let data = [0b1010_1100, 0b0111_0000];
/// let mut reader = BitReader::new(&data, 16);
///
/// assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
/// assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
/// assert_eq!(reader.read_bits(4).unwrap(), 0b0111);
/// assert_eq!(reader.remaining(), 4);
/// ```
#[derive(Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_len: usize,
    position: usize,
}

impl<'a> BitReader<'a> {
    /// Creates a new `BitReader` over the given byte slice.
    ///
    /// `bit_len` specifies the number of valid bits in `data`. It is clamped
    /// to `data.len() * 8` if larger.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::bit::BitReader;
    ///
    /// let data = [0b1010_0000];
    /// let mut reader = BitReader::new(&data, 4);
    /// assert_eq!(reader.remaining(), 4);
    /// assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
    /// ```
    pub fn new(data: &'a [u8], bit_len: usize) -> Self {
        Self {
            data,
            bit_len: bit_len.min(data.len() * 8),
            position: 0,
        }
    }

    /// Creates a new `BitReader` over the entire byte slice.
    ///
    /// All bits in `data` are considered valid (`bit_len = data.len() * 8`).
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self::new(data, data.len() * 8)
    }

    /// Returns the current bit position (0-indexed from the start).
    pub fn position(&self) -> usize {
        self.position
    }

    /// Returns the total number of valid bits.
    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// Returns the number of bits remaining to be read.
    pub fn remaining(&self) -> usize {
        self.bit_len - self.position
    }

    /// Returns `true` if there are no more bits to read.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Reads a single bit, returning `true` for 1 and `false` for 0.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::UnexpectedEof`] if no bits remain.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::bit::BitReader;
    ///
    /// let data = [0b1000_0000];
    /// let mut reader = BitReader::new(&data, 2);
    /// assert_eq!(reader.read_bit().unwrap(), true);
    /// assert_eq!(reader.read_bit().unwrap(), false);
    /// ```
    pub fn read_bit(&mut self) -> BitResult<bool> {
        if self.position >= self.bit_len {
            return Err(BitError::UnexpectedEof {
                position: self.position,
                requested: 1,
                remaining: 0,
            });
        }
        let byte_idx = self.position / 8;
        let bit_offset = self.position % 8;
        let bit = (self.data[byte_idx] >> (7 - bit_offset)) & 1;
        self.position += 1;
        Ok(bit != 0)
    }

    /// Reads `n` bits as an unsigned 64-bit integer (MSB-first).
    ///
    /// Reading 0 bits returns `Ok(0)`.
    ///
    /// # Errors
    ///
    /// - [`BitError::InvalidBitCount`] if `n > 64`.
    /// - [`BitError::UnexpectedEof`] if fewer than `n` bits remain.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::bit::BitReader;
    ///
    /// let data = [0b11010_011, 0b1_0000000];
    /// let mut reader = BitReader::new(&data, 9);
    /// assert_eq!(reader.read_bits(5).unwrap(), 0b11010);
    /// assert_eq!(reader.read_bits(4).unwrap(), 0b0111);
    /// ```
    pub fn read_bits(&mut self, n: u32) -> BitResult<u64> {
        if n == 0 {
            return Ok(0);
        }
        if n > 64 {
            return Err(BitError::InvalidBitCount { bits: n, max: 64 });
        }
        let n_bits = n as usize;
        let remaining = self.remaining();
        if n_bits > remaining {
            return Err(BitError::UnexpectedEof {
                position: self.position,
                requested: n_bits,
                remaining,
            });
        }

        let mut result: u64 = 0;
        let mut bits_left = n_bits;
        let mut pos = self.position;

        // Process bits in byte-aligned chunks for efficiency.
        while bits_left > 0 {
            let byte_idx = pos / 8;
            let bit_offset = pos % 8;
            let available = (8 - bit_offset).min(bits_left);

            // Shift the byte right so our target bits are at the LSB end,
            // then mask off just those bits.
            let shift = 8 - bit_offset - available;
            let mask = ((1u16 << available) - 1) as u8;
            let extracted = (self.data[byte_idx] >> shift) & mask;

            result = (result << available) | u64::from(extracted);
            pos += available;
            bits_left -= available;
        }

        self.position = pos;
        Ok(result)
    }

    /// Reads `n` bits as a `u8`.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 8`.
    /// Returns [`BitError::UnexpectedEof`] if fewer than `n` bits remain.
    pub fn read_u8(&mut self, n: u32) -> BitResult<u8> {
        if n > 8 {
            return Err(BitError::InvalidBitCount { bits: n, max: 8 });
        }
        self.read_bits(n).map(|v| v as u8)
    }

    /// Reads `n` bits as a `u16`.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 16`.
    /// Returns [`BitError::UnexpectedEof`] if fewer than `n` bits remain.
    pub fn read_u16(&mut self, n: u32) -> BitResult<u16> {
        if n > 16 {
            return Err(BitError::InvalidBitCount { bits: n, max: 16 });
        }
        self.read_bits(n).map(|v| v as u16)
    }

    /// Reads `n` bits as a `u32`.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::InvalidBitCount`] if `n > 32`.
    /// Returns [`BitError::UnexpectedEof`] if fewer than `n` bits remain.
    pub fn read_u32(&mut self, n: u32) -> BitResult<u32> {
        if n > 32 {
            return Err(BitError::InvalidBitCount { bits: n, max: 32 });
        }
        self.read_bits(n).map(|v| v as u32)
    }

    /// Reads `n` bits as a `u64`. Equivalent to [`read_bits`](Self::read_bits).
    pub fn read_u64(&mut self, n: u32) -> BitResult<u64> {
        self.read_bits(n)
    }

    /// Skips `n` bits, advancing the position without reading values.
    ///
    /// Skipping 0 bits is a no-op.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::UnexpectedEof`] if fewer than `n` bits remain.
    pub fn skip(&mut self, n: usize) -> BitResult<()> {
        if n == 0 {
            return Ok(());
        }
        let remaining = self.remaining();
        if n > remaining {
            return Err(BitError::UnexpectedEof {
                position: self.position,
                requested: n,
                remaining,
            });
        }
        self.position += n;
        Ok(())
    }

    /// Aligns the position to the next byte boundary.
    ///
    /// If already aligned, this is a no-op. Otherwise, skips the remaining
    /// bits in the current byte.
    ///
    /// # Errors
    ///
    /// Returns [`BitError::UnexpectedEof`] if there aren't enough bits to
    /// reach the next byte boundary.
    pub fn align_to_byte(&mut self) -> BitResult<()> {
        let remainder = self.position % 8;
        if remainder != 0 {
            self.skip(8 - remainder)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps_bit_len() {
        let data = [0xFF];
        let reader = BitReader::new(&data, 100);
        assert_eq!(reader.bit_len(), 8);
    }

    #[test]
    fn from_bytes_uses_full_length() {
        let data = [0xAB, 0xCD, 0xEF];
        let reader = BitReader::from_bytes(&data);
        assert_eq!(reader.bit_len(), 24);
    }

    #[test]
    fn empty_reader() {
        let data: [u8; 0] = [];
        let reader = BitReader::from_bytes(&data);
        assert!(reader.is_empty());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn read_bit_msb_first() {
        let data = [0b1010_0110];
        let mut reader = BitReader::from_bytes(&data);
        let bits: Vec<bool> = (0..8).map(|_| reader.read_bit().unwrap()).collect();
        assert_eq!(
            bits,
            vec![true, false, true, false, false, true, true, false]
        );
    }

    #[test]
    fn read_bit_at_eof() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data, 1);
        reader.read_bit().unwrap();
        let err = reader.read_bit().unwrap_err();
        assert_eq!(
            err,
            BitError::UnexpectedEof {
                position: 1,
                requested: 1,
                remaining: 0,
            }
        );
    }

    #[test]
    fn read_bits_zero_is_noop() {
        let data = [0xFF];
        let mut reader = BitReader::from_bytes(&data);
        assert_eq!(reader.read_bits(0).unwrap(), 0);
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn read_bits_within_byte() {
        let data = [0b1010_1100];
        let mut reader = BitReader::from_bytes(&data);
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        assert_eq!(reader.position(), 3);
        assert_eq!(reader.read_bits(5).unwrap(), 0b01100);
    }

    #[test]
    fn read_bits_crossing_boundary() {
        let data = [0b11010_011, 0b1_0000000];
        let mut reader = BitReader::new(&data, 9);
        assert_eq!(reader.read_bits(5).unwrap(), 0b11010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0111);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_full_u64() {
        let data = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let mut reader = BitReader::from_bytes(&data);
        assert_eq!(reader.read_u64(64).unwrap(), 0x0123456789ABCDEF);
    }

    #[test]
    fn read_bits_invalid_count() {
        let data = [0xFF; 16];
        let mut reader = BitReader::from_bytes(&data);
        let err = reader.read_bits(65).unwrap_err();
        assert_eq!(err, BitError::InvalidBitCount { bits: 65, max: 64 });
    }

    #[test]
    fn read_bits_past_eof() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data, 4);
        let err = reader.read_bits(5).unwrap_err();
        assert_eq!(
            err,
            BitError::UnexpectedEof {
                position: 0,
                requested: 5,
                remaining: 4,
            }
        );
    }

    #[test]
    fn read_u8_max_width() {
        let data = [0xAB];
        let mut reader = BitReader::from_bytes(&data);
        assert_eq!(reader.read_u8(8).unwrap(), 0xAB);
    }

    #[test]
    fn read_u8_too_wide() {
        let data = [0xFF; 2];
        let mut reader = BitReader::from_bytes(&data);
        let err = reader.read_u8(9).unwrap_err();
        assert_eq!(err, BitError::InvalidBitCount { bits: 9, max: 8 });
    }

    #[test]
    fn skip_advances_position() {
        let data = [0b1111_0000, 0b1010_1010];
        let mut reader = BitReader::from_bytes(&data);
        reader.skip(4).unwrap();
        assert_eq!(reader.position(), 4);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0000);
    }

    #[test]
    fn skip_zero_is_noop() {
        let data = [0xFF];
        let mut reader = BitReader::from_bytes(&data);
        reader.skip(0).unwrap();
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn skip_past_eof() {
        let data = [0xFF];
        let mut reader = BitReader::new(&data, 4);
        let err = reader.skip(5).unwrap_err();
        assert_eq!(
            err,
            BitError::UnexpectedEof {
                position: 0,
                requested: 5,
                remaining: 4,
            }
        );
    }

    #[test]
    fn align_from_unaligned() {
        let data = [0xFF, 0xAA];
        let mut reader = BitReader::from_bytes(&data);
        reader.read_bits(3).unwrap();
        reader.align_to_byte().unwrap();
        assert_eq!(reader.position(), 8);
        assert_eq!(reader.read_u8(8).unwrap(), 0xAA);
    }

    #[test]
    fn align_already_aligned() {
        let data = [0xFF, 0xAA];
        let mut reader = BitReader::from_bytes(&data);
        reader.read_bits(8).unwrap();
        reader.align_to_byte().unwrap();
        assert_eq!(reader.position(), 8);
    }

    #[test]
    fn align_at_start() {
        let data = [0xFF];
        let mut reader = BitReader::from_bytes(&data);
        reader.align_to_byte().unwrap();
        assert_eq!(reader.position(), 0);
    }
}
