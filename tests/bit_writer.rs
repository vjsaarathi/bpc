//! Integration tests for BitWriter.

use bpc::bit::{BitError, BitReader, BitWriter};

#[test]
fn write_single_bit_true() {
    let mut writer = BitWriter::new();
    writer.write_bit(true);
    writer.align_to_byte();
    assert_eq!(writer.into_bytes(), vec![0b1000_0000]);
}

#[test]
fn write_single_bit_false() {
    let mut writer = BitWriter::new();
    writer.write_bit(false);
    writer.align_to_byte();
    assert_eq!(writer.into_bytes(), vec![0b0000_0000]);
}

#[test]
fn write_multiple_bits_one_by_one() {
    let mut writer = BitWriter::new();
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
fn write_bits_crossing_byte_boundary() {
    let mut writer = BitWriter::new();
    writer.write_bits(0b10110, 5).unwrap();
    writer.write_bits(0b0110100, 7).unwrap();
    assert_eq!(writer.bit_len(), 12);
    writer.align_to_byte();
    assert_eq!(writer.into_bytes(), vec![0b1011_0011, 0b0100_0000]);
}

#[test]
fn write_full_byte() {
    let mut writer = BitWriter::new();
    writer.write_bits(0xAB, 8).unwrap();
    assert_eq!(writer.into_bytes(), vec![0xAB]);
}

#[test]
fn write_full_u16() {
    let mut writer = BitWriter::new();
    writer.write_u16(0xABCD, 16).unwrap();
    assert_eq!(writer.into_bytes(), vec![0xAB, 0xCD]);
}

#[test]
fn write_full_u32() {
    let mut writer = BitWriter::new();
    writer.write_u32(0x12345678, 32).unwrap();
    assert_eq!(writer.into_bytes(), vec![0x12, 0x34, 0x56, 0x78]);
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
    assert!(writer.into_bytes().is_empty());
}

#[test]
fn write_value_overflow_errors() {
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
    // 127 = 0b1111111 fits in 7 bits
    writer.write_bits(0b1111111, 7).unwrap();
    assert_eq!(writer.position(), 7);
}

#[test]
fn write_u8_max_in_8_bits() {
    let mut writer = BitWriter::new();
    writer.write_u8(u8::MAX, 8).unwrap();
    assert_eq!(writer.into_bytes(), vec![0xFF]);
}

#[test]
fn write_u8_max_in_7_bits_overflows() {
    let mut writer = BitWriter::new();
    let err = writer.write_u8(u8::MAX, 7).unwrap_err();
    assert_eq!(
        err,
        BitError::ValueOverflow {
            value: 255,
            bits: 7,
        }
    );
}

#[test]
fn write_bits_exceeds_64_errors() {
    let mut writer = BitWriter::new();
    let err = writer.write_bits(0, 65).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 65, max: 64 });
}

#[test]
fn write_u8_exceeds_8_errors() {
    let mut writer = BitWriter::new();
    let err = writer.write_u8(0, 9).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 9, max: 8 });
}

#[test]
fn write_u16_exceeds_16_errors() {
    let mut writer = BitWriter::new();
    let err = writer.write_u16(0, 17).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 17, max: 16 });
}

#[test]
fn write_u32_exceeds_32_errors() {
    let mut writer = BitWriter::new();
    let err = writer.write_u32(0, 33).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 33, max: 32 });
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
fn align_already_aligned_is_noop() {
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
fn position_tracks_correctly() {
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
fn as_bytes_reflects_current_state() {
    let mut writer = BitWriter::new();
    writer.write_bits(0xAB, 8).unwrap();
    assert_eq!(writer.as_bytes(), &[0xAB]);
    writer.write_bits(0xCD, 8).unwrap();
    assert_eq!(writer.as_bytes(), &[0xAB, 0xCD]);
}

// --- Round-trip tests ---

#[test]
fn round_trip_3_then_5_bits() {
    let mut writer = BitWriter::new();
    writer.write_bits(0b101, 3).unwrap();
    writer.write_bits(0b10110, 5).unwrap();
    let bytes = writer.into_bytes();

    let mut reader = BitReader::from_bytes(&bytes);
    assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    assert_eq!(reader.read_bits(5).unwrap(), 0b10110);
}

#[test]
fn round_trip_various_widths() {
    let mut writer = BitWriter::new();
    writer.write_bits(1, 1).unwrap();
    writer.write_bits(0b10, 2).unwrap();
    writer.write_bits(0b110, 3).unwrap();
    writer.write_bits(0xF, 4).unwrap();
    writer.write_bits(0b10101, 5).unwrap();
    writer.write_u8(0xAB, 8).unwrap();
    writer.write_u16(0x1234, 16).unwrap();

    let bit_len = writer.bit_len();
    let bytes = writer.into_bytes();

    let mut reader = BitReader::new(&bytes, bit_len);
    assert_eq!(reader.read_bits(1).unwrap(), 1);
    assert_eq!(reader.read_bits(2).unwrap(), 0b10);
    assert_eq!(reader.read_bits(3).unwrap(), 0b110);
    assert_eq!(reader.read_bits(4).unwrap(), 0xF);
    assert_eq!(reader.read_bits(5).unwrap(), 0b10101);
    assert_eq!(reader.read_u8(8).unwrap(), 0xAB);
    assert_eq!(reader.read_u16(16).unwrap(), 0x1234);
    assert!(reader.is_empty());
}

#[test]
fn round_trip_full_u64() {
    let mut writer = BitWriter::new();
    writer.write_u64(u64::MAX, 64).unwrap();
    let bytes = writer.into_bytes();

    let mut reader = BitReader::from_bytes(&bytes);
    assert_eq!(reader.read_u64(64).unwrap(), u64::MAX);
}

#[test]
fn round_trip_all_byte_values() {
    let mut writer = BitWriter::new();
    for v in 0..=255u8 {
        writer.write_u8(v, 8).unwrap();
    }
    let bytes = writer.into_bytes();

    let mut reader = BitReader::from_bytes(&bytes);
    for expected in 0..=255u8 {
        assert_eq!(reader.read_u8(8).unwrap(), expected);
    }
    assert!(reader.is_empty());
}

#[test]
fn round_trip_unaligned_sequence() {
    // Write 7-bit values to force constant boundary crossing.
    let values: Vec<u64> = (0..20).map(|i| i % 128).collect();
    let mut writer = BitWriter::new();
    for &v in &values {
        writer.write_bits(v, 7).unwrap();
    }

    let bit_len = writer.bit_len();
    let bytes = writer.into_bytes();

    let mut reader = BitReader::new(&bytes, bit_len);
    for &expected in &values {
        assert_eq!(reader.read_bits(7).unwrap(), expected);
    }
}
