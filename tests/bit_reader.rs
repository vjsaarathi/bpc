//! Integration tests for BitReader.

use bpc::bit::{BitError, BitReader};

#[test]
fn read_single_bit_msb() {
    let data = [0b1000_0000];
    let mut reader = BitReader::new(&data, 8);
    assert_eq!(reader.read_bit().unwrap(), true);
    assert_eq!(reader.position(), 1);
    assert_eq!(reader.remaining(), 7);
}

#[test]
fn read_single_bit_zero() {
    let data = [0b0111_1111];
    let mut reader = BitReader::new(&data, 8);
    assert_eq!(reader.read_bit().unwrap(), false);
}

#[test]
fn read_all_eight_bits() {
    let data = [0b1010_0110];
    let mut reader = BitReader::from_bytes(&data);
    let bits: Vec<bool> = (0..8).map(|_| reader.read_bit().unwrap()).collect();
    assert_eq!(
        bits,
        vec![true, false, true, false, false, true, true, false]
    );
    assert!(reader.is_empty());
}

#[test]
fn read_bits_within_single_byte() {
    let data = [0b1010_1100];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    assert_eq!(reader.read_bits(5).unwrap(), 0b01100);
}

#[test]
fn read_bits_crossing_byte_boundary() {
    let data = [0b1011_0011, 0b0100_1110];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_bits(5).unwrap(), 0b10110);
    assert_eq!(reader.read_bits(7).unwrap(), 0b0110100);
    assert_eq!(reader.remaining(), 4);
}

#[test]
fn read_aligned_bytes() {
    let data = [0xAB, 0xCD];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_bits(8).unwrap(), 0xAB);
    assert_eq!(reader.read_bits(8).unwrap(), 0xCD);
}

#[test]
fn read_u16_full_width() {
    let data = [0xAB, 0xCD];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_u16(16).unwrap(), 0xABCD);
}

#[test]
fn read_u32_full_width() {
    let data = [0x12, 0x34, 0x56, 0x78];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_u32(32).unwrap(), 0x12345678);
}

#[test]
fn read_u64_full_width() {
    let data = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_u64(64).unwrap(), 0x0123456789ABCDEF);
}

#[test]
fn read_zero_bits_returns_zero() {
    let data = [0xFF];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_bits(0).unwrap(), 0);
    assert_eq!(reader.position(), 0);
}

#[test]
fn read_bit_at_eof_errors() {
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
fn read_bits_past_eof_errors() {
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
fn read_bits_exceeds_64_errors() {
    let data = [0xFF; 16];
    let mut reader = BitReader::from_bytes(&data);
    let err = reader.read_bits(65).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 65, max: 64 });
}

#[test]
fn read_u8_exceeds_8_errors() {
    let data = [0xFF; 2];
    let mut reader = BitReader::from_bytes(&data);
    let err = reader.read_u8(9).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 9, max: 8 });
}

#[test]
fn read_u16_exceeds_16_errors() {
    let data = [0xFF; 4];
    let mut reader = BitReader::from_bytes(&data);
    let err = reader.read_u16(17).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 17, max: 16 });
}

#[test]
fn read_u32_exceeds_32_errors() {
    let data = [0xFF; 8];
    let mut reader = BitReader::from_bytes(&data);
    let err = reader.read_u32(33).unwrap_err();
    assert_eq!(err, BitError::InvalidBitCount { bits: 33, max: 32 });
}

#[test]
fn skip_bits() {
    let data = [0b1111_0000, 0b1010_1010];
    let mut reader = BitReader::from_bytes(&data);
    reader.skip(4).unwrap();
    assert_eq!(reader.position(), 4);
    assert_eq!(reader.read_bits(4).unwrap(), 0b0000);
    reader.skip(4).unwrap();
    assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
}

#[test]
fn skip_zero_is_noop() {
    let data = [0xFF];
    let mut reader = BitReader::from_bytes(&data);
    reader.skip(0).unwrap();
    assert_eq!(reader.position(), 0);
}

#[test]
fn skip_past_eof_errors() {
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
fn align_from_unaligned_position() {
    let data = [0b1110_0000, 0xFF];
    let mut reader = BitReader::from_bytes(&data);
    reader.read_bits(3).unwrap();
    reader.align_to_byte().unwrap();
    assert_eq!(reader.position(), 8);
    assert_eq!(reader.read_u8(8).unwrap(), 0xFF);
}

#[test]
fn align_already_aligned_is_noop() {
    let data = [0xFF, 0xAA];
    let mut reader = BitReader::from_bytes(&data);
    reader.read_bits(8).unwrap();
    reader.align_to_byte().unwrap();
    assert_eq!(reader.position(), 8);
}

#[test]
fn bit_len_clamped_to_data() {
    let data = [0xFF]; // only 8 bits
    let reader = BitReader::new(&data, 100);
    assert_eq!(reader.bit_len(), 8);
}

#[test]
fn partial_bit_len() {
    let data = [0b1010_0000];
    let mut reader = BitReader::new(&data, 4);
    assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
    assert!(reader.is_empty());
}

#[test]
fn empty_data() {
    let data: [u8; 0] = [];
    let reader = BitReader::from_bytes(&data);
    assert!(reader.is_empty());
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn read_three_then_five_bits() {
    // The example from the acceptance criteria.
    let data = [0b10110100];
    let mut reader = BitReader::from_bytes(&data);
    assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    assert_eq!(reader.read_bits(5).unwrap(), 0b10100);
}
