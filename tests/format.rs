//! Integration tests for data formatting and format registry.

use bpc::format::{FieldFormatter, FormatContext, FormatId, FormatRegistry};
use bpc::layout::BitLayout;
use bpc::tui::LayoutViewState;

struct CustomPrefixFormatter;

impl FieldFormatter for CustomPrefixFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("custom_prefix")
    }

    fn name(&self) -> &str {
        "Custom Prefix"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        match ctx.numeric_value() {
            Some(v) => format!("VAL#{v}"),
            None => "VAL#?".to_string(),
        }
    }
}

#[test]
fn test_custom_formatter_registration() {
    let mut reg = FormatRegistry::with_builtins();
    reg.register(CustomPrefixFormatter);

    assert!(reg.get(&FormatId::new("custom_prefix")).is_some());
    assert_eq!(
        reg.get(&FormatId::new("custom_prefix")).unwrap().name(),
        "Custom Prefix"
    );

    let data = vec![0x2A];
    let ctx = FormatContext {
        data: &data,
        offset: 0,
        width: 8,
        parsed_value: Some(bpc::format::Value::Primitive(42)),
        field_type: None,
    };
    let formatted = reg.get(&FormatId::new("custom_prefix")).unwrap().format(&ctx);
    assert_eq!(formatted, "VAL#42");
}

#[test]
fn test_view_state_field_and_global_toggle() {
    let layout = BitLayout::builder()
        .field("ver", 4)
        .field("type", 4)
        .build()
        .unwrap();

    let data = vec![0xAB]; // ver = 0xA (10), type = 0xB (11)
    let mut view = LayoutViewState::new(layout, data);

    // Initial global format should be "hex"
    assert_eq!(view.global_format().as_str(), "hex");
    assert_eq!(view.field_format("ver").as_str(), "hex");
    assert_eq!(view.field_format("type").as_str(), "hex");

    assert_eq!(view.format_field_value("ver"), "0xA");
    assert_eq!(view.format_field_value("type"), "0xB");

    // Toggle selected field (index 0 is selected by default)
    view.toggle_selected_field_format();
    assert_eq!(view.field_format("ver").as_str(), "dec");
    assert_eq!(view.field_format("type").as_str(), "hex"); // field 1 still hex
    assert_eq!(view.format_field_value("ver"), "10");
    assert_eq!(view.format_field_value("type"), "0xB");

    // Toggle global format: advances hex -> dec, and resets per-field overrides
    view.toggle_global_format();
    assert_eq!(view.global_format().as_str(), "dec");
    assert_eq!(view.field_format("ver").as_str(), "dec");
    assert_eq!(view.field_format("type").as_str(), "dec");
    assert_eq!(view.format_field_value("ver"), "10");
    assert_eq!(view.format_field_value("type"), "11");

    // Toggle global format to binary
    view.toggle_global_format();
    assert_eq!(view.global_format().as_str(), "bin");
    assert_eq!(view.format_field_value("ver"), "0b1010");
    assert_eq!(view.format_field_value("type"), "0b1011");
}

#[test]
fn test_custom_registry_in_view_state() {
    let mut reg = FormatRegistry::new();
    reg.register(CustomPrefixFormatter);

    let layout = BitLayout::builder().field("field1", 8).build().unwrap();
    let data = vec![123];
    let view = LayoutViewState::with_registry(layout, data, reg);

    assert_eq!(view.global_format().as_str(), "custom_prefix");
    assert_eq!(view.format_field_value("field1"), "VAL#123");
}
