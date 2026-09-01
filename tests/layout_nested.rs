use bpc::layout::{BitLayout, LengthUnit};
use std::sync::Arc;

#[test]
fn test_nested_layout_basic() {
    let header = BitLayout::builder()
        .field("source", 16)
        .field("dest", 16)
        .build()
        .unwrap();

    let packet = BitLayout::builder()
        .field("version", 8)
        .field_layout("header", Arc::new(header))
        .field("payload", 32)
        .build()
        .unwrap();

    assert_eq!(packet.bit_len(), 8 + 32 + 32);
    
    // Check paths
    let (v, abs) = packet.find_by_path("version").unwrap();
    assert_eq!(v.width(), 8);
    assert_eq!(abs, 0);

    let (h, abs) = packet.find_by_path("header").unwrap();
    assert_eq!(h.width(), 32);
    assert_eq!(abs, 8);

    let (src, abs) = packet.find_by_path("header.source").unwrap();
    assert_eq!(src.width(), 16);
    assert_eq!(abs, 8);

    let (dst, abs) = packet.find_by_path("header.dest").unwrap();
    assert_eq!(dst.width(), 16);
    assert_eq!(abs, 24);

    let (p, abs) = packet.find_by_path("payload").unwrap();
    assert_eq!(p.width(), 32);
    assert_eq!(abs, 40);
}

#[test]
fn test_invalid_dot_in_name() {
    let err = BitLayout::builder()
        .field("header.version", 8)
        .build()
        .unwrap_err();
    assert_eq!(err.to_string(), "field name \"header.version\" is invalid (cannot contain '.')");
}

#[test]
fn test_nested_layout_resolve() {
    // Nested layout has a variable field
    let header = BitLayout::builder()
        .field("len", 8)
        .field_var("data", "len", bpc::layout::LengthUnit::Bytes)
        .build()
        .unwrap();

    let packet = BitLayout::builder()
        .field("version", 8)
        .field_layout("header", Arc::new(header))
        .field("checksum", 16)
        .build()
        .unwrap();

    // Data: version=1, header.len=2 (2 bytes), header.data=[0xAA, 0xBB], checksum=0x1234
    // total bits = 8 + (8 + 16) + 16 = 48 (6 bytes)
    let data = vec![
        0x01, // version
        0x02, // len
        0xAA, 0xBB, // data
        0x12, 0x34, // checksum
    ];

    let resolved = packet.resolve(&data).unwrap();

    assert_eq!(resolved.bit_len(), 48);

    let (v, abs) = resolved.find_by_path("version").unwrap();
    assert_eq!(v.width(), 8);
    assert_eq!(abs, 0);

    let (h, abs) = resolved.find_by_path("header").unwrap();
    assert_eq!(h.width(), 24); // 8 + 16
    assert_eq!(abs, 8);

    let (d, abs) = resolved.find_by_path("header.data").unwrap();
    assert_eq!(d.width(), 16);
    assert_eq!(abs, 16); // 8(version) + 8(len)

    let (c, abs) = resolved.find_by_path("checksum").unwrap();
    assert_eq!(c.width(), 16);
    assert_eq!(abs, 32); // 8(version) + 24(header)
}
