//! Bit range and layout field types.
//!
//! All bit offsets are zero-based. Bit ranges use half-open intervals
//! `[offset, offset + width)`. Widths are measured in bits.

/// A contiguous range of bits using half-open interval `[offset, offset + width)`.
///
/// # Convention
///
/// Given `offset = 3` and `width = 5`:
/// - The range covers bits `3, 4, 5, 6, 7`.
/// - `end()` returns `8` (exclusive).
///
/// # Examples
///
/// ```
/// use bpc::layout::BitRange;
///
/// let r = BitRange::new(3, 5);
/// assert_eq!(r.offset(), 3);
/// assert_eq!(r.width(), 5);
/// assert_eq!(r.end(), 8);
/// assert!(r.contains(3));
/// assert!(r.contains(7));
/// assert!(!r.contains(8));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitRange {
    offset: usize,
    width: usize,
}

impl BitRange {
    /// Creates a new bit range starting at `offset` with the given `width`.
    pub fn new(offset: usize, width: usize) -> Self {
        Self { offset, width }
    }

    /// Starting bit offset (inclusive).
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Number of bits in this range.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Exclusive end position: `offset + width`.
    pub fn end(&self) -> usize {
        self.offset + self.width
    }

    /// Returns `true` if this range contains the given bit offset.
    ///
    /// Uses half-open semantics: `offset <= bit < end()`.
    pub fn contains(&self, bit: usize) -> bool {
        bit >= self.offset && bit < self.end()
    }

    /// Returns `true` if this range overlaps with `other`.
    pub fn overlaps(&self, other: &BitRange) -> bool {
        self.offset < other.end() && other.offset < self.end()
    }
}

/// The unit used when interpreting a length field's value.
///
/// When a field's width is derived from another field, the source field's
/// runtime value is interpreted in this unit to compute the target width in bits.
///
/// # Examples
///
/// ```
/// use bpc::layout::LengthUnit;
///
/// // A "length" field with value 4 and unit Bytes means 4 * 8 = 32 bits.
/// assert_eq!(LengthUnit::Bytes.to_bits(4), 32);
/// assert_eq!(LengthUnit::Bits.to_bits(4), 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// The source field value is already in bits.
    Bits,
    /// The source field value is in bytes (multiplied by 8 for bit width).
    Bytes,
}

impl LengthUnit {
    /// Converts a raw value in this unit to a bit count.
    pub fn to_bits(self, value: u64) -> u64 {
        match self {
            LengthUnit::Bits => value,
            LengthUnit::Bytes => value * 8,
        }
    }
}

/// Describes how a field's width is determined.
///
/// A field can have a fixed width known at build time, or a variable width
/// whose size is derived from the runtime value of a previously declared field.
///
/// # Examples
///
/// ```
/// use bpc::layout::{FieldWidth, LengthUnit};
///
/// let fixed = FieldWidth::Fixed(16);
/// assert!(fixed.is_fixed());
///
/// let variable = FieldWidth::DerivedFrom {
///     source_field: "length".to_string(),
///     unit: LengthUnit::Bytes,
/// };
/// assert!(!variable.is_fixed());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldWidth {
    /// A fixed width known at layout-build time.
    Fixed(usize),
    /// Width derived from another field's runtime value.
    ///
    /// The `source_field` must be declared before this field in the layout.
    /// At resolve time, the source field's value (read from data) is converted
    /// via `unit` to compute the width in bits.
    DerivedFrom {
        /// Name of the field whose value determines this field's width.
        source_field: String,
        /// Unit the source value is expressed in.
        unit: LengthUnit,
    },
}

impl FieldWidth {
    /// Returns `true` if this is a fixed width.
    pub fn is_fixed(&self) -> bool {
        matches!(self, FieldWidth::Fixed(_))
    }

    /// Returns the fixed width value, or `None` if derived.
    pub fn fixed_width(&self) -> Option<usize> {
        match self {
            FieldWidth::Fixed(w) => Some(*w),
            FieldWidth::DerivedFrom { .. } => None,
        }
    }
}

/// The type of a field in a layout.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// A primitive bit field, with either a fixed or variable width.
    Primitive(FieldWidth),
    /// A nested layout. The width is intrinsically the layout's total bit length.
    Layout(std::sync::Arc<crate::layout::BitLayout>),
    /// An enumeration mapping encoded values to variants.
    Enum {
        width: FieldWidth,
        // Scaffolded for now.
        // mapping: Arc<HashMap<u64, String>>
    },
}

impl FieldType {
    /// Returns `true` if this field type has a fixed width.
    pub fn is_fixed(&self) -> bool {
        match self {
            FieldType::Primitive(w) | FieldType::Enum { width: w, .. } => w.is_fixed(),
            FieldType::Layout(_) => true, // Layouts are currently fixed-width aggregations
        }
    }

    /// Returns the fixed width value, or `None` if derived.
    pub fn fixed_width(&self) -> Option<usize> {
        match self {
            FieldType::Primitive(w) | FieldType::Enum { width: w, .. } => w.fixed_width(),
            FieldType::Layout(l) => Some(l.bit_len()),
        }
    }
}

/// A named region in a bit stream.
///
/// Identifies a field by name and bit range. Does not carry protocol-specific
/// semantics — it simply marks a named region.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutField {
    name: String,
    offset: usize,
    field_type: FieldType,
}

impl LayoutField {
    /// Creates a new primitive layout field with the given name and bit range.
    ///
    /// The field is assumed to have a fixed width matching the range.
    pub fn new(name: impl Into<String>, range: BitRange) -> Self {
        Self {
            name: name.into(),
            offset: range.offset(),
            field_type: FieldType::Primitive(FieldWidth::Fixed(range.width())),
        }
    }

    /// Creates a new primitive layout field with a variable width specification.
    pub fn new_variable(
        name: impl Into<String>,
        offset: usize,
        width_spec: FieldWidth,
    ) -> Self {
        Self {
            name: name.into(),
            offset,
            field_type: FieldType::Primitive(width_spec),
        }
    }

    /// Creates a field representing a nested layout.
    pub fn new_layout(name: impl Into<String>, offset: usize, layout: std::sync::Arc<crate::layout::BitLayout>) -> Self {
        Self {
            name: name.into(),
            offset,
            field_type: FieldType::Layout(layout),
        }
    }

    /// The field's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The field's type.
    pub fn field_type(&self) -> &FieldType {
        &self.field_type
    }

    /// Returns `true` if this field has a variable (data-dependent) width.
    pub fn is_variable(&self) -> bool {
        !self.field_type.is_fixed()
    }

    /// Starting bit offset.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Width in bits. If variable and unresolved, returns 0.
    pub fn width(&self) -> usize {
        self.field_type.fixed_width().unwrap_or(0)
    }

    /// The field's bit range (computed on the fly).
    pub fn range(&self) -> BitRange {
        BitRange::new(self.offset(), self.width())
    }

    /// Exclusive end position.
    pub fn end(&self) -> usize {
        self.offset() + self.width()
    }

    /// Returns `true` if this field contains the given bit offset.
    pub fn contains(&self, bit: usize) -> bool {
        self.range().contains(bit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_range_basics() {
        let r = BitRange::new(3, 5);
        assert_eq!(r.offset(), 3);
        assert_eq!(r.width(), 5);
        assert_eq!(r.end(), 8);
    }

    #[test]
    fn bit_range_contains_half_open() {
        let r = BitRange::new(3, 5);
        assert!(!r.contains(2));
        assert!(r.contains(3));
        assert!(r.contains(4));
        assert!(r.contains(7));
        assert!(!r.contains(8)); // exclusive end
    }

    #[test]
    fn bit_range_contains_at_zero() {
        let r = BitRange::new(0, 3);
        assert!(r.contains(0));
        assert!(r.contains(2));
        assert!(!r.contains(3));
    }

    #[test]
    fn bit_range_overlaps_partial() {
        let a = BitRange::new(0, 8);
        let b = BitRange::new(4, 8);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn bit_range_no_overlap_adjacent() {
        let a = BitRange::new(0, 8);
        let b = BitRange::new(8, 8);
        assert!(!a.overlaps(&b));
        assert!(!b.overlaps(&a));
    }

    #[test]
    fn bit_range_no_overlap_gap() {
        let a = BitRange::new(0, 4);
        let b = BitRange::new(8, 4);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn bit_range_overlap_exact() {
        let a = BitRange::new(0, 8);
        let b = BitRange::new(0, 8);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn layout_field_accessors() {
        let field = LayoutField::new("version", BitRange::new(0, 3));
        assert_eq!(field.name(), "version");
        assert_eq!(field.offset(), 0);
        assert_eq!(field.width(), 3);
        assert_eq!(field.end(), 3);
    }

    #[test]
    fn layout_field_contains() {
        let field = LayoutField::new("opcode", BitRange::new(3, 5));
        assert!(!field.contains(2));
        assert!(field.contains(3));
        assert!(field.contains(7));
        assert!(!field.contains(8));
    }

    #[test]
    fn layout_field_range_accessor() {
        let range = BitRange::new(10, 20);
        let field = LayoutField::new("data", range);
        assert_eq!(field.range(), range);
    }
}
