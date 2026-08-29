//! Property-based tests for the bit engine.

use bpc::bit::{BitReader, BitWriter};
use proptest::prelude::*;

proptest! {
    #[test]
    fn round_trip_single_bits(bits in prop::collection::vec(any::<bool>(), 0..200)) {
        let mut writer = BitWriter::new();
        for &b in &bits {
            writer.write_bit(b);
        }
        let bit_len = writer.bit_len();
        let bytes = writer.into_bytes();

        let mut reader = BitReader::new(&bytes, bit_len);
        for &expected in &bits {
            let actual = reader.read_bit().unwrap();
            prop_assert_eq!(actual, expected);
        }
        prop_assert!(reader.is_empty());
    }

    #[test]
    fn round_trip_byte_values(values in prop::collection::vec(any::<u8>(), 1..50)) {
        let mut writer = BitWriter::new();
        for &v in &values {
            writer.write_u8(v, 8).unwrap();
        }
        let bytes = writer.into_bytes();

        let mut reader = BitReader::from_bytes(&bytes);
        for &expected in &values {
            let actual = reader.read_u8(8).unwrap();
            prop_assert_eq!(actual, expected);
        }
    }

    #[test]
    fn round_trip_variable_width(
        widths_and_values in prop::collection::vec(
            (1u32..=16, any::<u16>()),
            1..30
        )
    ) {
        let mut writer = BitWriter::new();
        let mut expected_values = Vec::new();

        for &(width, value) in &widths_and_values {
            let max_val = if width >= 16 { u16::MAX } else { (1u16 << width) - 1 };
            let clamped = value & max_val;
            writer.write_u16(clamped, width).unwrap();
            expected_values.push((width, clamped));
        }

        let bit_len = writer.bit_len();
        let bytes = writer.into_bytes();
        let mut reader = BitReader::new(&bytes, bit_len);

        for &(width, expected) in &expected_values {
            let actual = reader.read_u16(width).unwrap();
            prop_assert_eq!(actual, expected, "width={}", width);
        }
    }

    #[test]
    fn position_advances_correctly(ops in prop::collection::vec(1u32..=8, 1..50)) {
        let total_bits: u32 = ops.iter().sum();
        if total_bits > 1024 {
            return Ok(());
        }

        let mut writer = BitWriter::new();
        let mut expected_pos = 0usize;

        for &width in &ops {
            writer.write_bits(0, width).unwrap();
            expected_pos += width as usize;
            prop_assert_eq!(writer.position(), expected_pos);
        }

        let bit_len = writer.bit_len();
        let bytes = writer.into_bytes();
        let mut reader = BitReader::new(&bytes, bit_len);
        expected_pos = 0;

        for &width in &ops {
            reader.read_bits(width).unwrap();
            expected_pos += width as usize;
            prop_assert_eq!(reader.position(), expected_pos);
        }
    }

    #[test]
    fn remaining_decreases_monotonically(
        widths in prop::collection::vec(1u32..=8, 1..30)
    ) {
        let total: usize = widths.iter().map(|w| *w as usize).sum();
        if total > 256 {
            return Ok(());
        }

        // Build a buffer of zeros with enough bits.
        let mut writer = BitWriter::new();
        for &w in &widths {
            writer.write_bits(0, w).unwrap();
        }
        let bit_len = writer.bit_len();
        let bytes = writer.into_bytes();

        let mut reader = BitReader::new(&bytes, bit_len);
        let mut prev_remaining = reader.remaining();

        for &w in &widths {
            reader.read_bits(w).unwrap();
            let cur = reader.remaining();
            prop_assert!(cur < prev_remaining || w == 0);
            prev_remaining = cur;
        }
    }
}
