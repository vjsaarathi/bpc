//! Extensible data formatting system for fields and bit layouts.
//!
//! Provides the [`FieldFormatter`] trait and [`FormatRegistry`] allowing
//! values to be rendered in different bases (hex, dec, bin, oct), strings (ASCII/UTF-8),
//! or custom registered formats.

use std::collections::HashMap;
use std::sync::Arc;

/// A format identifier used for lookup in the registry and selection in views.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FormatId(pub String);

impl FormatId {
    /// Creates a new format ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the string slice of the ID.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for FormatId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for FormatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context provided to a formatter when rendering a field or bit range.
#[derive(Debug, Clone)]
pub struct FormatContext<'a> {
    /// The raw buffer containing the data.
    pub data: &'a [u8],
    /// Bit offset of the field.
    pub offset: usize,
    /// Width of the field in bits.
    pub width: usize,
    /// Extracted integer value (if <= 64 bits and fully available).
    pub numeric_value: Option<u64>,
}

/// Trait for formatting bitfield data into human-readable representations.
pub trait FieldFormatter: Send + Sync {
    /// Unique identifier for this formatter (e.g. "hex", "dec", "bin", "oct", "ascii").
    fn id(&self) -> FormatId;

    /// Human-friendly display label (e.g. "Hexadecimal", "Decimal", "Binary", "ASCII").
    fn name(&self) -> &str;

    /// Formats the field for display.
    fn format(&self, ctx: &FormatContext) -> String;

    /// Formats a compact representation suitable for table/diagram labels if needed.
    fn format_compact(&self, ctx: &FormatContext) -> String {
        self.format(ctx)
    }
}

// Built-in formatters:

/// Hexadecimal formatter (e.g. `0x1F`, `0x00A4`).
#[derive(Debug, Default, Clone, Copy)]
pub struct HexFormatter;

impl FieldFormatter for HexFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("hex")
    }

    fn name(&self) -> &str {
        "Hex (0x)"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        match ctx.numeric_value {
            Some(val) => {
                let nibbles = (ctx.width.saturating_add(3) / 4).max(1);
                format!("0x{val:0nibbles$X}")
            }
            None => extract_bits_string(ctx.data, ctx.offset, ctx.width)
                .map(|b| format!("(raw: {b})"))
                .unwrap_or_else(|| "(unavailable)".to_string()),
        }
    }
}

/// Unsigned Decimal formatter (e.g. `123`).
#[derive(Debug, Default, Clone, Copy)]
pub struct DecFormatter;

impl FieldFormatter for DecFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("dec")
    }

    fn name(&self) -> &str {
        "Decimal"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        match ctx.numeric_value {
            Some(val) => format!("{val}"),
            None => "(unavailable)".to_string(),
        }
    }
}

/// Binary formatter (e.g. `0b10110`).
#[derive(Debug, Default, Clone, Copy)]
pub struct BinFormatter;

impl FieldFormatter for BinFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("bin")
    }

    fn name(&self) -> &str {
        "Binary (0b)"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        match ctx.numeric_value {
            Some(val) => {
                let width = ctx.width.max(1);
                format!("0b{val:0width$b}")
            }
            None => extract_bits_string(ctx.data, ctx.offset, ctx.width)
                .map(|b| format!("0b{b}"))
                .unwrap_or_else(|| "(unavailable)".to_string()),
        }
    }
}

/// Octal formatter (e.g. `0o755`).
#[derive(Debug, Default, Clone, Copy)]
pub struct OctFormatter;

impl FieldFormatter for OctFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("oct")
    }

    fn name(&self) -> &str {
        "Octal (0o)"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        match ctx.numeric_value {
            Some(val) => {
                let oct_digits = (ctx.width.saturating_add(2) / 3).max(1);
                format!("0o{val:0oct_digits$o}")
            }
            None => "(unavailable)".to_string(),
        }
    }
}

/// ASCII/String formatter (interprets bytes as ASCII characters or escaped chars).
#[derive(Debug, Default, Clone, Copy)]
pub struct AsciiFormatter;

impl FieldFormatter for AsciiFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("ascii")
    }

    fn name(&self) -> &str {
        "ASCII / Text"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        if ctx.width % 8 == 0 && ctx.offset % 8 == 0 {
            let start_byte = ctx.offset / 8;
            let byte_len = ctx.width / 8;
            if start_byte + byte_len <= ctx.data.len() {
                let slice = &ctx.data[start_byte..start_byte + byte_len];
                let mut out = String::with_capacity(byte_len);
                for &b in slice {
                    if b.is_ascii_graphic() || b == b' ' {
                        out.push(b as char);
                    } else {
                        out.push_str(&format!("\\x{b:02X}"));
                    }
                }
                return format!("\"{out}\"");
            }
        }

        // For non-byte-aligned or integer values:
        match ctx.numeric_value {
            Some(val) => {
                let bytes = val.to_be_bytes();
                let needed_bytes = (ctx.width + 7) / 8;
                let start = 8usize.saturating_sub(needed_bytes);
                let slice = &bytes[start..];
                let mut out = String::new();
                for &b in slice {
                    if b.is_ascii_graphic() || b == b' ' {
                        out.push(b as char);
                    } else {
                        out.push_str(&format!("\\x{b:02X}"));
                    }
                }
                format!("\"{out}\"")
            }
            None => "(unavailable)".to_string(),
        }
    }
}

/// Formatter creating raw bit sequence string (e.g. `10110010`).
#[derive(Debug, Default, Clone, Copy)]
pub struct RawBitsFormatter;

impl FieldFormatter for RawBitsFormatter {
    fn id(&self) -> FormatId {
        FormatId::new("raw")
    }

    fn name(&self) -> &str {
        "Raw Bits"
    }

    fn format(&self, ctx: &FormatContext) -> String {
        extract_bits_string(ctx.data, ctx.offset, ctx.width).unwrap_or_else(|| "(unavailable)".to_string())
    }
}

fn extract_bits_string(data: &[u8], offset: usize, width: usize) -> Option<String> {
    if offset + width > data.len() * 8 {
        return None;
    }
    let mut out = String::with_capacity(width);
    for i in 0..width {
        let bit_pos = offset + i;
        let byte_idx = bit_pos / 8;
        let bit_in_byte = 7 - (bit_pos % 8);
        let val = (data[byte_idx] >> bit_in_byte) & 1;
        out.push(if val == 1 { '1' } else { '0' });
    }
    Some(out)
}

/// Registry of available data formatters.
#[derive(Clone)]
pub struct FormatRegistry {
    formatters: Vec<Arc<dyn FieldFormatter>>,
    by_id: HashMap<FormatId, usize>,
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

impl FormatRegistry {
    /// Creates an empty format registry.
    pub fn new() -> Self {
        Self {
            formatters: Vec::new(),
            by_id: HashMap::new(),
        }
    }

    /// Creates a registry initialized with all built-in formatters:
    /// Hex, Dec, Bin, Oct, ASCII, Raw Bits.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(HexFormatter);
        reg.register(DecFormatter);
        reg.register(BinFormatter);
        reg.register(OctFormatter);
        reg.register(AsciiFormatter);
        reg.register(RawBitsFormatter);
        reg
    }

    /// Registers a new formatter into the registry.
    pub fn register<F: FieldFormatter + 'static>(&mut self, formatter: F) {
        let id = formatter.id();
        let idx = self.formatters.len();
        self.formatters.push(Arc::new(formatter));
        self.by_id.insert(id, idx);
    }

    /// Registers a shared formatter.
    pub fn register_arc(&mut self, formatter: Arc<dyn FieldFormatter>) {
        let id = formatter.id();
        let idx = self.formatters.len();
        self.formatters.push(formatter);
        self.by_id.insert(id, idx);
    }

    /// Returns all registered formatters in order.
    pub fn formatters(&self) -> &[Arc<dyn FieldFormatter>] {
        &self.formatters
    }

    /// Returns the number of registered formatters.
    pub fn len(&self) -> usize {
        self.formatters.len()
    }

    /// Returns `true` if no formatters are registered.
    pub fn is_empty(&self) -> bool {
        self.formatters.is_empty()
    }

    /// Retrieves a formatter by its index.
    pub fn get_by_index(&self, index: usize) -> Option<&Arc<dyn FieldFormatter>> {
        self.formatters.get(index)
    }

    /// Retrieves a formatter by its ID.
    pub fn get(&self, id: &FormatId) -> Option<&Arc<dyn FieldFormatter>> {
        self.by_id.get(id).and_then(|&idx| self.formatters.get(idx))
    }

    /// Returns the next format ID in cyclic order after the given ID.
    pub fn next_format_id(&self, current: &FormatId) -> Option<FormatId> {
        if self.formatters.is_empty() {
            return None;
        }
        let current_idx = self.by_id.get(current).copied().unwrap_or(0);
        let next_idx = (current_idx + 1) % self.formatters.len();
        Some(self.formatters[next_idx].id())
    }

    /// Returns the previous format ID in cyclic order before the given ID.
    pub fn prev_format_id(&self, current: &FormatId) -> Option<FormatId> {
        if self.formatters.is_empty() {
            return None;
        }
        let current_idx = self.by_id.get(current).copied().unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            self.formatters.len() - 1
        } else {
            current_idx - 1
        };
        Some(self.formatters[prev_idx].id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtins_present() {
        let reg = FormatRegistry::with_builtins();
        assert_eq!(reg.len(), 6);
        assert!(reg.get(&FormatId::new("hex")).is_some());
        assert!(reg.get(&FormatId::new("dec")).is_some());
        assert!(reg.get(&FormatId::new("bin")).is_some());
        assert!(reg.get(&FormatId::new("oct")).is_some());
        assert!(reg.get(&FormatId::new("ascii")).is_some());
        assert!(reg.get(&FormatId::new("raw")).is_some());
    }

    #[test]
    fn test_cycle_formats() {
        let reg = FormatRegistry::with_builtins();
        let hex = FormatId::new("hex");
        let dec = reg.next_format_id(&hex).unwrap();
        assert_eq!(dec, FormatId::new("dec"));
        let prev = reg.prev_format_id(&dec).unwrap();
        assert_eq!(prev, hex);
    }

    #[test]
    fn test_hex_formatting() {
        let formatter = HexFormatter;
        let ctx = FormatContext {
            data: &[0xAB, 0xCD],
            offset: 0,
            width: 16,
            numeric_value: Some(0xABCD),
        };
        assert_eq!(formatter.format(&ctx), "0xABCD");
    }

    #[test]
    fn test_dec_formatting() {
        let formatter = DecFormatter;
        let ctx = FormatContext {
            data: &[0x00, 0x2A],
            offset: 0,
            width: 16,
            numeric_value: Some(42),
        };
        assert_eq!(formatter.format(&ctx), "42");
    }

    #[test]
    fn test_bin_formatting() {
        let formatter = BinFormatter;
        let ctx = FormatContext {
            data: &[0b1010],
            offset: 0,
            width: 4,
            numeric_value: Some(0b1010),
        };
        assert_eq!(formatter.format(&ctx), "0b1010");
    }

    #[test]
    fn test_ascii_formatting() {
        let formatter = AsciiFormatter;
        let ctx = FormatContext {
            data: b"OK",
            offset: 0,
            width: 16,
            numeric_value: Some(0x4F4B),
        };
        assert_eq!(formatter.format(&ctx), "\"OK\"");
    }
}
