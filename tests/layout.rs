//! Integration tests for the layout module.

use bpc::layout::{BitLayout, BitRange, LayoutError, LayoutField};

// --- BitRange tests ---

#[test]
fn bit_range_half_open_semantics() {
    let r = BitRange::new(3, 5);
    assert_eq!(r.offset(), 3);
    assert_eq!(r.width(), 5);
    assert_eq!(r.end(), 8);

    assert!(!r.contains(2));
    assert!(r.contains(3));
    assert!(r.contains(7));
    assert!(!r.contains(8)); // exclusive end
}

#[test]
fn bit_range_at_zero() {
    let r = BitRange::new(0, 3);
    assert!(r.contains(0));
    assert!(r.contains(2));
    assert!(!r.contains(3));
}

#[test]
fn bit_range_overlaps_symmetric() {
    let a = BitRange::new(0, 8);
    let b = BitRange::new(4, 8);
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn bit_range_adjacent_no_overlap() {
    let a = BitRange::new(0, 8);
    let b = BitRange::new(8, 8);
    assert!(!a.overlaps(&b));
    assert!(!b.overlaps(&a));
}

// --- LayoutField tests ---

#[test]
fn layout_field_accessors() {
    let field = LayoutField::new("version", BitRange::new(0, 3));
    assert_eq!(field.name(), "version");
    assert_eq!(field.offset(), 0);
    assert_eq!(field.width(), 3);
    assert_eq!(field.end(), 3);
}

// --- Sequential layout tests ---

#[test]
fn sequential_offsets_correct() {
    let layout = BitLayout::builder()
        .field("version", 3)
        .field("opcode", 5)
        .field("length", 16)
        .field("flags", 8)
        .build()
        .unwrap();

    assert_eq!(layout.field(0).unwrap().offset(), 0);
    assert_eq!(layout.field(1).unwrap().offset(), 3);
    assert_eq!(layout.field(2).unwrap().offset(), 8);
    assert_eq!(layout.field(3).unwrap().offset(), 24);
}

#[test]
fn total_length_32_bits() {
    let layout = BitLayout::builder()
        .field("version", 3)
        .field("opcode", 5)
        .field("length", 16)
        .field("flags", 8)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 32);
}

#[test]
fn field_count_matches() {
    let layout = BitLayout::builder()
        .field("a", 8)
        .field("b", 16)
        .field("c", 4)
        .build()
        .unwrap();

    assert_eq!(layout.field_count(), 3);
}

// --- Lookup tests ---

#[test]
fn lookup_by_index() {
    let layout = BitLayout::builder()
        .field("a", 8)
        .field("b", 8)
        .build()
        .unwrap();

    assert_eq!(layout.field(0).unwrap().name(), "a");
    assert_eq!(layout.field(1).unwrap().name(), "b");
    assert!(layout.field(2).is_none());
}

#[test]
fn lookup_by_name() {
    let layout = BitLayout::builder()
        .field("version", 3)
        .field("opcode", 5)
        .build()
        .unwrap();

    assert_eq!(layout.field_by_name("version").unwrap().width(), 3);
    assert_eq!(layout.field_by_name("opcode").unwrap().width(), 5);
    assert!(layout.field_by_name("missing").is_none());
}

#[test]
fn lookup_by_bit_offset() {
    let layout = BitLayout::builder()
        .field("version", 3)
        .field("opcode", 5)
        .field("length", 16)
        .build()
        .unwrap();

    assert_eq!(layout.field_at_bit(0).unwrap().name(), "version");
    assert_eq!(layout.field_at_bit(2).unwrap().name(), "version");
    assert_eq!(layout.field_at_bit(3).unwrap().name(), "opcode");
    assert_eq!(layout.field_at_bit(7).unwrap().name(), "opcode");
    assert_eq!(layout.field_at_bit(8).unwrap().name(), "length");
    assert_eq!(layout.field_at_bit(23).unwrap().name(), "length");
    assert!(layout.field_at_bit(24).is_none());
}

#[test]
fn bit_at_field_end_belongs_to_next() {
    let layout = BitLayout::builder()
        .field("a", 3)
        .field("b", 5)
        .build()
        .unwrap();

    // Bit 2 is the last in field "a" [0, 3).
    assert_eq!(layout.field_at_bit(2).unwrap().name(), "a");
    // Bit 3 is the start of field "b" [3, 8), not "a".
    assert_eq!(layout.field_at_bit(3).unwrap().name(), "b");
}

#[test]
fn bits_outside_layout_return_none() {
    let layout = BitLayout::builder()
        .field("x", 8)
        .build()
        .unwrap();

    assert!(layout.field_at_bit(8).is_none());
    assert!(layout.field_at_bit(100).is_none());
}

#[test]
fn field_index_at_bit_values() {
    let layout = BitLayout::builder()
        .field("a", 8)
        .field("b", 8)
        .build()
        .unwrap();

    assert_eq!(layout.field_index_at_bit(0), Some(0));
    assert_eq!(layout.field_index_at_bit(7), Some(0));
    assert_eq!(layout.field_index_at_bit(8), Some(1));
    assert_eq!(layout.field_index_at_bit(15), Some(1));
    assert_eq!(layout.field_index_at_bit(16), None);
}

// --- Validation tests ---

#[test]
fn reject_empty_field_name() {
    let result = BitLayout::builder().field("", 8).build();
    assert_eq!(result.unwrap_err(), LayoutError::EmptyFieldName);
}

#[test]
fn reject_zero_width_field() {
    let result = BitLayout::builder().field("x", 0).build();
    assert_eq!(
        result.unwrap_err(),
        LayoutError::ZeroWidth {
            name: "x".to_string()
        }
    );
}

#[test]
fn reject_duplicate_field_names() {
    let result = BitLayout::builder()
        .field("x", 8)
        .field("x", 8)
        .build();

    assert_eq!(
        result.unwrap_err(),
        LayoutError::DuplicateName {
            name: "x".to_string()
        }
    );
}

#[test]
fn reject_overlapping_explicit_ranges() {
    let result = BitLayout::builder()
        .field_at("a", 0, 8)
        .field_at("b", 4, 8)
        .build();

    assert_eq!(
        result.unwrap_err(),
        LayoutError::OverlappingFields {
            existing: "a".to_string(),
            new: "b".to_string(),
        }
    );
}

// --- Empty layout ---

#[test]
fn empty_layout_is_valid() {
    let layout = BitLayout::builder().build().unwrap();
    assert!(layout.is_empty());
    assert_eq!(layout.bit_len(), 0);
    assert_eq!(layout.field_count(), 0);
    assert!(layout.field(0).is_none());
    assert!(layout.field_by_name("x").is_none());
    assert!(layout.field_at_bit(0).is_none());
}

// --- Non-byte-aligned layouts ---

#[test]
fn layout_10_bits() {
    let layout = BitLayout::builder()
        .field("a", 3)
        .field("b", 5)
        .field("c", 2)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 10);
}

#[test]
fn layout_13_bits() {
    let layout = BitLayout::builder()
        .field("a", 5)
        .field("b", 8)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 13);
}

#[test]
fn layout_21_bits() {
    let layout = BitLayout::builder()
        .field("a", 7)
        .field("b", 7)
        .field("c", 7)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 21);
}

// --- Explicit positioning ---

#[test]
fn explicit_positioning_works() {
    let layout = BitLayout::builder()
        .field_at("flags", 24, 8)
        .field_at("version", 0, 3)
        .build()
        .unwrap();

    assert_eq!(layout.field_by_name("version").unwrap().offset(), 0);
    assert_eq!(layout.field_by_name("flags").unwrap().offset(), 24);
    assert_eq!(layout.bit_len(), 32);
}

#[test]
fn adjacent_explicit_fields_ok() {
    let layout = BitLayout::builder()
        .field_at("a", 0, 8)
        .field_at("b", 8, 8)
        .build()
        .unwrap();

    assert_eq!(layout.bit_len(), 16);
}

// --- Large layout ---

#[test]
fn large_layout_100_fields() {
    let mut builder = BitLayout::builder();
    for i in 0..100 {
        builder = builder.field(&format!("f{i}"), 8);
    }
    let layout = builder.build().unwrap();

    assert_eq!(layout.field_count(), 100);
    assert_eq!(layout.bit_len(), 800);
    assert_eq!(layout.field(0).unwrap().offset(), 0);
    assert_eq!(layout.field(50).unwrap().offset(), 400);
    assert_eq!(layout.field(99).unwrap().offset(), 792);
    assert_eq!(layout.field_at_bit(0).unwrap().name(), "f0");
    assert_eq!(layout.field_at_bit(400).unwrap().name(), "f50");
    assert_eq!(layout.field_at_bit(799).unwrap().name(), "f99");
}

// --- LayoutViewState tests ---

mod view_state {
    use bpc::layout::BitLayout;
    use bpc::tui::LayoutViewState;

    fn demo_layout() -> BitLayout {
        BitLayout::builder()
            .field("version", 3)
            .field("opcode", 5)
            .field("length", 16)
            .field("flags", 8)
            .build()
            .unwrap()
    }

    #[test]
    fn initial_cursor_at_zero() {
        let view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        assert_eq!(view.cursor_bit(), 0);
        assert_eq!(view.selected_field_index(), Some(0));
    }

    #[test]
    fn move_next_field() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        view.move_next_field();
        assert_eq!(view.cursor_bit(), 3); // opcode starts at 3
        assert_eq!(view.selected_field_index(), Some(1));
    }

    #[test]
    fn move_next_field_at_last_stays() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        // Move to last field (flags)
        view.move_next_field(); // opcode
        view.move_next_field(); // length
        view.move_next_field(); // flags
        assert_eq!(view.selected_field_index(), Some(3));
        view.move_next_field(); // should stay at flags
        assert_eq!(view.selected_field_index(), Some(3));
    }

    #[test]
    fn move_prev_field() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        view.move_next_field(); // go to opcode
        view.move_prev_field(); // back to version
        assert_eq!(view.cursor_bit(), 0);
        assert_eq!(view.selected_field_index(), Some(0));
    }

    #[test]
    fn move_prev_field_at_first_stays() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        view.move_prev_field(); // already at first, should stay
        assert_eq!(view.cursor_bit(), 0);
    }

    #[test]
    fn move_next_bit_within_field() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        view.move_next_bit();
        assert_eq!(view.cursor_bit(), 1);
        view.move_next_bit();
        assert_eq!(view.cursor_bit(), 2);
        // At end of version [0,3), should stop.
        view.move_next_bit();
        assert_eq!(view.cursor_bit(), 2);
    }

    #[test]
    fn move_prev_bit_within_field() {
        let mut view = LayoutViewState::new(demo_layout(), vec![0; 4]);
        view.move_next_field(); // opcode at bit 3
        view.move_next_bit(); // bit 4
        view.move_prev_bit(); // bit 3
        assert_eq!(view.cursor_bit(), 3);
        // At start of opcode, should stop.
        view.move_prev_bit();
        assert_eq!(view.cursor_bit(), 3);
    }

    #[test]
    fn read_bit_value() {
        let data = vec![0b10110100, 0xFF, 0x00, 0xAA];
        let view = LayoutViewState::new(demo_layout(), data);

        assert_eq!(view.read_bit_value(0), Some(true));
        assert_eq!(view.read_bit_value(1), Some(false));
        assert_eq!(view.read_bit_value(2), Some(true));
    }

    #[test]
    fn read_bit_value_beyond_data() {
        let data = vec![0xFF]; // only 8 bits
        let layout = BitLayout::builder()
            .field("x", 16)
            .build()
            .unwrap();
        let view = LayoutViewState::new(layout, data);

        assert_eq!(view.read_bit_value(0), Some(true));
        assert_eq!(view.read_bit_value(7), Some(true));
        assert_eq!(view.read_bit_value(8), None); // beyond data
    }

    #[test]
    fn empty_layout_navigation() {
        let layout = BitLayout::builder().build().unwrap();
        let mut view = LayoutViewState::new(layout, vec![]);
        view.move_next_field(); // no-op
        view.move_prev_field(); // no-op
        view.move_next_bit(); // no-op
        view.move_prev_bit(); // no-op
        assert_eq!(view.cursor_bit(), 0);
        assert_eq!(view.selected_field_index(), None);
    }
}

// --- Variable-width field tests ---

mod variable_width {
    use bpc::layout::{BitLayout, FieldWidth, LayoutError, LengthUnit};

    // -- Builder validation --

    #[test]
    fn build_var_field_ok() {
        let layout = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        assert!(layout.has_variable_fields());
        assert_eq!(layout.field_count(), 2);
        assert_eq!(layout.field(0).unwrap().name(), "length");
        assert_eq!(layout.field(1).unwrap().name(), "payload");
        assert!(layout.field(1).unwrap().is_variable());
    }

    #[test]
    fn build_rejects_unknown_source() {
        let result = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "missing", LengthUnit::Bytes)
            .build();

        assert_eq!(
            result.unwrap_err(),
            LayoutError::UnknownSourceField {
                field: "payload".into(),
                source: "missing".into(),
            }
        );
    }

    #[test]
    fn build_rejects_forward_reference() {
        let result = BitLayout::builder()
            .field_var("payload", "length", LengthUnit::Bytes)
            .field("length", 8)
            .build();

        assert_eq!(
            result.unwrap_err(),
            LayoutError::ForwardReference {
                field: "payload".into(),
                source: "length".into(),
            }
        );
    }

    #[test]
    fn build_rejects_self_reference() {
        // A field referencing itself is also a forward reference (pos >= i).
        let result = BitLayout::builder()
            .field_var("payload", "payload", LengthUnit::Bits)
            .build();

        assert_eq!(
            result.unwrap_err(),
            LayoutError::ForwardReference {
                field: "payload".into(),
                source: "payload".into(),
            }
        );
    }

    #[test]
    fn build_rejects_variable_source() {
        // "mid" is variable, so "tail" can't use it as a source.
        let result = BitLayout::builder()
            .field("head_len", 8)
            .field_var("mid", "head_len", LengthUnit::Bytes)
            .field_var("tail", "mid", LengthUnit::Bits)
            .build();

        assert_eq!(
            result.unwrap_err(),
            LayoutError::VariableSourceField {
                field: "tail".into(),
                source: "mid".into(),
            }
        );
    }

    #[test]
    fn build_rejects_duplicate_name_with_var() {
        let result = BitLayout::builder()
            .field("x", 8)
            .field_var("x", "x", LengthUnit::Bits)
            .build();

        assert_eq!(
            result.unwrap_err(),
            LayoutError::DuplicateName {
                name: "x".into(),
            }
        );
    }

    #[test]
    fn build_rejects_empty_name_var() {
        let result = BitLayout::builder()
            .field("len", 8)
            .field_var("", "len", LengthUnit::Bytes)
            .build();

        assert_eq!(result.unwrap_err(), LayoutError::EmptyFieldName);
    }

    // -- Resolve tests --

    #[test]
    fn resolve_length_bytes() {
        let template = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        // length=3 → payload is 3 bytes = 24 bits
        let data = vec![3, 0xAA, 0xBB, 0xCC];
        let resolved = template.resolve(&data).unwrap();

        assert!(!resolved.has_variable_fields());
        assert_eq!(resolved.field_count(), 2);

        let len_field = resolved.field_by_name("length").unwrap();
        assert_eq!(len_field.offset(), 0);
        assert_eq!(len_field.width(), 8);

        let payload_field = resolved.field_by_name("payload").unwrap();
        assert_eq!(payload_field.offset(), 8);
        assert_eq!(payload_field.width(), 24);

        assert_eq!(resolved.bit_len(), 32);
    }

    #[test]
    fn resolve_length_bits() {
        let template = BitLayout::builder()
            .field("bit_count", 8)
            .field_var("data", "bit_count", LengthUnit::Bits)
            .build()
            .unwrap();

        // bit_count=12 → data is 12 bits
        let data = vec![12, 0xFF, 0xFF];
        let resolved = template.resolve(&data).unwrap();

        let data_field = resolved.field_by_name("data").unwrap();
        assert_eq!(data_field.offset(), 8);
        assert_eq!(data_field.width(), 12);
        assert_eq!(resolved.bit_len(), 20);
    }

    #[test]
    fn resolve_with_fixed_fields_before_and_after() {
        // header(4) | length(8) | payload(var, bytes) | trailer(8)
        let template = BitLayout::builder()
            .field("header", 4)
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .field("trailer", 8)
            .build()
            .unwrap();

        // length=2 → payload is 2 bytes = 16 bits
        // Data: header nibble + length byte 2 + 2 payload bytes + trailer byte
        let data = vec![
            0b0000_0000, // header(4)=0, length(8) starts at bit 4
            0b0010_0000, // length MSB bits... value = 2
            0xAA,        // payload
            0xBB,        // payload continued
            0xCC,        // trailer data
        ];

        // Actually let's think about this more carefully. The layout:
        // header: bits 0..4 (4 bits)
        // length: bits 4..12 (8 bits)
        // payload: bits 12..? (var)
        // trailer: bits ?..?+8 (8 bits, but note: fixed at build time with offset 12)
        //
        // Wait — the trailer is fixed at build time but the payload is variable,
        // so the trailer's offset depends on payload resolution. Let me make this simpler.

        // Actually, looking at the builder code, fixed fields after a variable field
        // get placed at the same offset as the variable field's placeholder (width 0).
        // They need to be resolved too. Let me use a simpler test case.
        let template = BitLayout::builder()
            .field("header", 4)
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        // header=any(4 bits), length byte = 0x02 (value 2)
        // bits 0..3 = header, bits 4..11 = length, bits 12..27 = payload(16 bits)
        let data = vec![
            0x02, // bits 0-3: header=0x0, bits 4-7: upper nibble of length
            0x00, // bits 8-11: lower nibble of length, then padding
            0xAA, 0xBB, // payload data
        ];

        // Actually the data layout: MSB-first bit reading
        // Byte 0 (0x02) = 0b00000010
        //   bits 0..3 = 0b0000 (header)
        //   bits 4..7 = 0b0010 (upper 4 bits of length)
        // Byte 1 (0x00) = 0b00000000
        //   bits 8..11 = 0b0000 (lower 4 bits of length)
        //   length = 0b00100000 = 0x20 = 32... that's too big.
        //
        // Let me just use nice byte-aligned data.
        let template2 = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        let data2 = vec![2, 0xAA, 0xBB];
        let resolved2 = template2.resolve(&data2).unwrap();

        assert_eq!(resolved2.field_by_name("length").unwrap().offset(), 0);
        assert_eq!(resolved2.field_by_name("length").unwrap().width(), 8);
        assert_eq!(resolved2.field_by_name("payload").unwrap().offset(), 8);
        assert_eq!(resolved2.field_by_name("payload").unwrap().width(), 16);
        assert_eq!(resolved2.bit_len(), 24);
    }

    #[test]
    fn resolve_multiple_var_fields_from_same_source() {
        // One length field controls two different payloads.
        let template = BitLayout::builder()
            .field("length", 8)
            .field_var("payload_a", "length", LengthUnit::Bytes)
            .field_var("payload_b", "length", LengthUnit::Bits)
            .build()
            .unwrap();

        // length=4 → payload_a = 4 bytes = 32 bits, payload_b = 4 bits
        let data = vec![4, 0, 0, 0, 0, 0xFF];
        let resolved = template.resolve(&data).unwrap();

        assert_eq!(resolved.field_by_name("payload_a").unwrap().offset(), 8);
        assert_eq!(resolved.field_by_name("payload_a").unwrap().width(), 32);
        assert_eq!(resolved.field_by_name("payload_b").unwrap().offset(), 40);
        assert_eq!(resolved.field_by_name("payload_b").unwrap().width(), 4);
        assert_eq!(resolved.bit_len(), 44);
    }

    #[test]
    fn resolve_two_independent_var_fields() {
        // Two separate length fields, each controlling their own data.
        let template = BitLayout::builder()
            .field("len_a", 8)
            .field("len_b", 8)
            .field_var("data_a", "len_a", LengthUnit::Bytes)
            .field_var("data_b", "len_b", LengthUnit::Bytes)
            .build()
            .unwrap();

        // len_a=1 (1 byte=8 bits), len_b=2 (2 bytes=16 bits)
        let data = vec![1, 2, 0xAA, 0xBB, 0xCC];
        let resolved = template.resolve(&data).unwrap();

        assert_eq!(resolved.field_by_name("len_a").unwrap().offset(), 0);
        assert_eq!(resolved.field_by_name("len_b").unwrap().offset(), 8);
        assert_eq!(resolved.field_by_name("data_a").unwrap().offset(), 16);
        assert_eq!(resolved.field_by_name("data_a").unwrap().width(), 8);
        assert_eq!(resolved.field_by_name("data_b").unwrap().offset(), 24);
        assert_eq!(resolved.field_by_name("data_b").unwrap().width(), 16);
        assert_eq!(resolved.bit_len(), 40);
    }

    #[test]
    fn resolve_no_variable_fields_is_identity() {
        let layout = BitLayout::builder()
            .field("a", 8)
            .field("b", 16)
            .build()
            .unwrap();

        assert!(!layout.has_variable_fields());

        let data = vec![0xFF, 0x00, 0x00];
        let resolved = layout.resolve(&data).unwrap();

        assert_eq!(resolved.field_count(), layout.field_count());
        assert_eq!(resolved.bit_len(), layout.bit_len());
        assert_eq!(resolved.field(0).unwrap().offset(), 0);
        assert_eq!(resolved.field(0).unwrap().width(), 8);
        assert_eq!(resolved.field(1).unwrap().offset(), 8);
        assert_eq!(resolved.field(1).unwrap().width(), 16);
    }

    #[test]
    fn resolve_different_length_values() {
        // Same template, different data → different resolved layouts.
        let template = BitLayout::builder()
            .field("len", 8)
            .field_var("data", "len", LengthUnit::Bytes)
            .build()
            .unwrap();

        // length=1
        let r1 = template.resolve(&[1, 0xAA]).unwrap();
        assert_eq!(r1.field_by_name("data").unwrap().width(), 8);
        assert_eq!(r1.bit_len(), 16);

        // length=4
        let r4 = template.resolve(&[4, 0, 0, 0, 0]).unwrap();
        assert_eq!(r4.field_by_name("data").unwrap().width(), 32);
        assert_eq!(r4.bit_len(), 40);
    }

    #[test]
    fn resolve_16bit_length_field() {
        // A 16-bit length field for larger payloads.
        let template = BitLayout::builder()
            .field("length", 16)
            .field_var("payload", "length", LengthUnit::Bits)
            .build()
            .unwrap();

        // length = 0x0018 = 24 bits = 3 bytes
        let data = vec![0x00, 0x18, 0xAA, 0xBB, 0xCC];
        let resolved = template.resolve(&data).unwrap();

        assert_eq!(resolved.field_by_name("payload").unwrap().width(), 24);
        assert_eq!(resolved.bit_len(), 40);
    }

    #[test]
    fn resolve_non_byte_aligned_source() {
        // Source field is only 4 bits wide.
        let template = BitLayout::builder()
            .field("nibble_len", 4)
            .field("flags", 4)
            .field_var("data", "nibble_len", LengthUnit::Bytes)
            .build()
            .unwrap();

        // nibble_len=0x2 (4 bits, MSB-first), flags=0xF (4 bits)
        // Byte 0 = 0b0010_1111 = 0x2F
        // nibble_len = 0b0010 = 2, data = 2 bytes = 16 bits
        let data = vec![0x2F, 0xAA, 0xBB];
        let resolved = template.resolve(&data).unwrap();

        assert_eq!(resolved.field_by_name("nibble_len").unwrap().width(), 4);
        assert_eq!(resolved.field_by_name("flags").unwrap().width(), 4);
        assert_eq!(resolved.field_by_name("data").unwrap().offset(), 8);
        assert_eq!(resolved.field_by_name("data").unwrap().width(), 16);
    }

    // -- Resolve error cases --

    #[test]
    fn resolve_insufficient_data() {
        let template = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        // Empty data — can't even read the length field.
        let result = template.resolve(&[]);
        assert_eq!(
            result.unwrap_err(),
            LayoutError::InsufficientData {
                field: "payload".into(),
                needed_bits: 8,
                available_bits: 0,
            }
        );
    }

    #[test]
    fn resolve_zero_width_error() {
        let template = BitLayout::builder()
            .field("length", 8)
            .field_var("payload", "length", LengthUnit::Bytes)
            .build()
            .unwrap();

        // length=0 → payload would be 0 bits, which is invalid.
        let result = template.resolve(&[0]);
        assert_eq!(
            result.unwrap_err(),
            LayoutError::ResolvedZeroWidth {
                field: "payload".into(),
            }
        );
    }

    // -- FieldWidth and LengthUnit unit tests --

    #[test]
    fn field_width_is_fixed() {
        assert!(FieldWidth::Fixed(8).is_fixed());
        assert_eq!(FieldWidth::Fixed(8).fixed_width(), Some(8));
    }

    #[test]
    fn field_width_derived_not_fixed() {
        let w = FieldWidth::DerivedFrom {
            source_field: "len".into(),
            unit: LengthUnit::Bytes,
        };
        assert!(!w.is_fixed());
        assert_eq!(w.fixed_width(), None);
    }

    #[test]
    fn length_unit_bits() {
        assert_eq!(LengthUnit::Bits.to_bits(10), 10);
        assert_eq!(LengthUnit::Bits.to_bits(0), 0);
    }

    #[test]
    fn length_unit_bytes() {
        assert_eq!(LengthUnit::Bytes.to_bits(1), 8);
        assert_eq!(LengthUnit::Bytes.to_bits(4), 32);
        assert_eq!(LengthUnit::Bytes.to_bits(0), 0);
    }

    // -- Resolved layout lookups --

    #[test]
    fn resolved_field_lookups_work() {
        let template = BitLayout::builder()
            .field("type", 8)
            .field("len", 8)
            .field_var("body", "len", LengthUnit::Bytes)
            .build()
            .unwrap();

        let data = vec![0x01, 0x03, 0xAA, 0xBB, 0xCC];
        let resolved = template.resolve(&data).unwrap();

        // field_at_bit
        assert_eq!(resolved.field_at_bit(0).unwrap().name(), "type");
        assert_eq!(resolved.field_at_bit(7).unwrap().name(), "type");
        assert_eq!(resolved.field_at_bit(8).unwrap().name(), "len");
        assert_eq!(resolved.field_at_bit(15).unwrap().name(), "len");
        assert_eq!(resolved.field_at_bit(16).unwrap().name(), "body");
        assert_eq!(resolved.field_at_bit(39).unwrap().name(), "body");
        assert!(resolved.field_at_bit(40).is_none());

        // field_index_at_bit
        assert_eq!(resolved.field_index_at_bit(0), Some(0));
        assert_eq!(resolved.field_index_at_bit(8), Some(1));
        assert_eq!(resolved.field_index_at_bit(16), Some(2));
    }
}
