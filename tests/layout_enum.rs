use bpc::layout::{BitLayout, FieldWidth, LengthUnit};
use bpc::layout::enum_def::{EnumDef, EnumVariant};
use std::sync::Arc;

#[test]
fn test_enum_basic() {
    let variants = vec![
        EnumVariant { name: "IPv4".into(), value: 4 },
        EnumVariant { name: "IPv6".into(), value: 6 },
    ];
    let proto_enum = EnumDef::new("Protocol", FieldWidth::Fixed(4), variants).unwrap();

    let layout = BitLayout::builder()
        .field_enum("version", Arc::new(proto_enum))
        .field("ihl", 4)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 8);
    let (v, abs) = layout.find_by_path("version").unwrap();
    assert_eq!(v.width(), 4);
    assert_eq!(abs, 0);

    if let bpc::layout::field::FieldType::Enum(e) = v.field_type() {
        assert_eq!(e.variant_name(4), Some("IPv4"));
        assert_eq!(e.variant_name(6), Some("IPv6"));
        assert_eq!(e.variant_name(5), None);
        
        assert_eq!(e.variant_value("IPv4"), Some(4));
    } else {
        panic!("Not an enum");
    }
}

#[test]
fn test_enum_var_width() {
    let variants = vec![
        EnumVariant { name: "A".into(), value: 1 },
        EnumVariant { name: "B".into(), value: 2 },
    ];
    let dyn_enum = EnumDef::new(
        "DynEnum",
        FieldWidth::DerivedFrom { source_field: "len".into(), unit: LengthUnit::Bytes },
        variants
    ).unwrap();

    let layout = BitLayout::builder()
        .field("len", 8)
        .field_enum("type", Arc::new(dyn_enum))
        .build()
        .unwrap();

    // length is 2 bytes -> type enum is 16 bits
    let data = vec![0x02, 0x00, 0x02];
    let resolved = layout.resolve(&data).unwrap();

    assert_eq!(resolved.bit_len(), 24);
    let (t, abs) = resolved.find_by_path("type").unwrap();
    assert_eq!(t.width(), 16);
    assert_eq!(abs, 8);
    
    if let bpc::layout::field::FieldType::Enum(e) = t.field_type() {
        assert_eq!(e.variant_name(2), Some("B"));
    } else {
        panic!("Not an enum");
    }
}
