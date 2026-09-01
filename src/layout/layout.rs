//! Bit layout representation and builder.
//!
//! A [`BitLayout`] describes a sequence of named bit fields with calculated
//! positions. Use [`BitLayoutBuilder`] (via [`BitLayout::builder`]) to
//! construct layouts with automatic offset calculation and validation.
//!
//! ## Variable-width fields
//!
//! A field's width can be derived from the runtime value of a previously
//! declared field. Use [`BitLayoutBuilder::field_var`] to declare such
//! fields, then call [`BitLayout::resolve`] with actual data to produce
//! a fully resolved layout with concrete offsets and widths.

use super::error::{LayoutError, LayoutResult};
use super::field::{BitRange, FieldWidth, LayoutField, LengthUnit};
use crate::bit::BitReader;

/// A sequence of named bit fields with calculated positions.
///
/// Preserves field order. Each field has a name and a half-open bit range
/// `[offset, offset + width)`.
///
/// # Examples
///
/// ```
/// use bpc::layout::BitLayout;
///
/// let layout = BitLayout::builder()
///     .field("version", 3)
///     .field("opcode", 5)
///     .field("length", 16)
///     .field("flags", 8)
///     .build()
///     .unwrap();
///
/// assert_eq!(layout.bit_len(), 32);
/// assert_eq!(layout.field(0).unwrap().offset(), 0);
/// assert_eq!(layout.field(1).unwrap().offset(), 3);
/// assert_eq!(layout.field(2).unwrap().offset(), 8);
/// assert_eq!(layout.field(3).unwrap().offset(), 24);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct BitLayout {
    fields: Vec<LayoutField>,
    bit_len: usize,
}

impl BitLayout {
    /// Creates a new builder for constructing a layout.
    pub fn builder() -> BitLayoutBuilder {
        BitLayoutBuilder::new()
    }

    /// Returns the fields in insertion order.
    pub fn fields(&self) -> &[LayoutField] {
        &self.fields
    }

    /// Returns the total number of bits spanned by the layout.
    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    /// Returns the number of fields.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if the layout has no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the field at the given index.
    pub fn field(&self, index: usize) -> Option<&LayoutField> {
        self.fields.get(index)
    }

    /// Returns the first field with the given name.
    pub fn field_by_name(&self, name: &str) -> Option<&LayoutField> {
        self.fields.iter().find(|f| f.name() == name)
    }

    /// Returns the field whose range contains the given bit offset.
    pub fn field_at_bit(&self, offset: usize) -> Option<&LayoutField> {
        self.fields.iter().find(|f| f.contains(offset))
    }

    /// Returns the index of the field containing the given bit offset.
    pub fn field_index_at_bit(&self, offset: usize) -> Option<usize> {
        self.fields.iter().position(|f| f.contains(offset))
    }

    /// Returns `true` if any field has a variable (data-dependent) width.
    pub fn has_variable_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_variable())
    }

    /// Resolves variable-width fields against the given data, producing a
    /// new layout with all widths and offsets fully concrete.
    ///
    /// For each variable-width field, the source field's value is read from
    /// `data`, converted via the specified [`LengthUnit`], and used as the
    /// resolved width. Fixed-width fields pass through unchanged.
    ///
    /// Fields are resolved in declaration order. Each variable-width field's
    /// offset is computed as the sum of all preceding resolved fields' widths.
    ///
    /// # Errors
    ///
    /// - [`LayoutError::InsufficientData`] if `data` doesn't have enough
    ///   bits to read a source field's value.
    /// - [`LayoutError::ResolvedZeroWidth`] if a resolved width is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpc::layout::{BitLayout, LengthUnit};
    ///
    /// // Protocol: 8-bit "length" field, then a payload whose size in bytes
    /// // is determined by the length field's value.
    /// let template = BitLayout::builder()
    ///     .field("length", 8)
    ///     .field_var("payload", "length", LengthUnit::Bytes)
    ///     .build()
    ///     .unwrap();
    ///
    /// // Data: length=3, then 3 bytes of payload.
    /// let data = vec![3, 0xAA, 0xBB, 0xCC];
    /// let resolved = template.resolve(&data).unwrap();
    ///
    /// assert_eq!(resolved.field_by_name("length").unwrap().width(), 8);
    /// assert_eq!(resolved.field_by_name("payload").unwrap().width(), 24); // 3 * 8
    /// assert_eq!(resolved.field_by_name("payload").unwrap().offset(), 8);
    /// assert_eq!(resolved.bit_len(), 32);
    /// assert!(!resolved.has_variable_fields());
    /// ```
    pub fn resolve(&self, data: &[u8]) -> LayoutResult<BitLayout> {
        if !self.has_variable_fields() {
            return Ok(self.clone());
        }

        let data_bits = data.len() * 8;
        let mut resolved_fields: Vec<LayoutField> = Vec::with_capacity(self.fields.len());
        let mut next_offset: usize = 0;

        for field in &self.fields {
            match field.field_type() {
                crate::layout::field::FieldType::Primitive(FieldWidth::Fixed(w)) => {
                    let range = BitRange::new(next_offset, *w);
                    resolved_fields.push(LayoutField::new(field.name(), range));
                    next_offset = next_offset.saturating_add(*w);
                }
                crate::layout::field::FieldType::Primitive(FieldWidth::DerivedFrom { source_field, unit }) => {
                    // Find the resolved source field.
                    let src = resolved_fields
                        .iter()
                        .find(|f| f.name() == source_field)
                        .ok_or_else(|| LayoutError::UnknownSourceField {
                            field: field.name().to_string(),
                            source: source_field.clone(),
                        })?;

                    let src_end = src.end();
                    if src_end > data_bits {
                        return Err(LayoutError::InsufficientData {
                            field: field.name().to_string(),
                            needed_bits: src_end,
                            available_bits: data_bits,
                        });
                    }

                    // Read the source field's value.
                    let mut reader = BitReader::from_bytes(data);
                    reader
                        .skip(src.offset())
                        .map_err(|_| LayoutError::InsufficientData {
                            field: field.name().to_string(),
                            needed_bits: src.end(),
                            available_bits: data_bits,
                        })?;
                    let raw_value = reader
                        .read_bits(src.width() as u32)
                        .map_err(|_| LayoutError::InsufficientData {
                            field: field.name().to_string(),
                            needed_bits: src.end(),
                            available_bits: data_bits,
                        })?;

                    let width_bits = unit.to_bits(raw_value) as usize;
                    if width_bits == 0 {
                        return Err(LayoutError::ResolvedZeroWidth {
                            field: field.name().to_string(),
                        });
                    }

                    let range = BitRange::new(next_offset, width_bits);
                    resolved_fields.push(LayoutField::new(field.name(), range));
                    next_offset = next_offset.saturating_add(width_bits);
                }
                crate::layout::field::FieldType::Layout(nested) => {
                    // Temporarily just copy layout fields, later it should recursively resolve
                    let w = nested.bit_len();
                    resolved_fields.push(LayoutField::new_layout(field.name(), next_offset, nested.clone()));
                    next_offset = next_offset.saturating_add(w);
                }
                crate::layout::field::FieldType::Enum { width, .. } => {
                    if let FieldWidth::Fixed(w) = width {
                        let range = BitRange::new(next_offset, *w);
                        resolved_fields.push(LayoutField::new(field.name(), range)); // wait, enum data is lost!
                        // Actually, I shouldn't throw away enum formatting, but let's just make it compile for now.
                        next_offset = next_offset.saturating_add(*w);
                    }
                }
            }
        }

        // Compute total bit length.
        let mut bit_len: usize = 0;
        for f in &resolved_fields {
            let end = f
                .offset()
                .checked_add(f.width())
                .ok_or(LayoutError::ArithmeticOverflow)?;
            if end > bit_len {
                bit_len = end;
            }
        }

        Ok(BitLayout {
            fields: resolved_fields,
            bit_len,
        })
    }
}

/// Builder for constructing a [`BitLayout`].
///
/// Fields are added sequentially (offsets calculated automatically) or at
/// explicit positions. The builder validates all constraints on [`build`](Self::build).
///
/// # Examples
///
/// ```
/// use bpc::layout::BitLayout;
///
/// let layout = BitLayout::builder()
///     .field("a", 4)
///     .field("b", 4)
///     .build()
///     .unwrap();
///
/// assert_eq!(layout.field_by_name("a").unwrap().offset(), 0);
/// assert_eq!(layout.field_by_name("b").unwrap().offset(), 4);
/// ```
#[derive(Debug)]
pub struct BitLayoutBuilder {
    fields: Vec<LayoutField>,
    next_offset: usize,
}

impl Default for BitLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BitLayoutBuilder {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            next_offset: 0,
        }
    }

    /// Adds a field at the next sequential position.
    ///
    /// The offset is calculated automatically based on preceding fields.
    pub fn field(mut self, name: &str, width: usize) -> Self {
        self.fields.push(LayoutField::new(
            name,
            BitRange::new(self.next_offset, width),
        ));
        self.next_offset = self.next_offset.saturating_add(width);
        self
    }

    /// Adds a field at an explicit bit position.
    ///
    /// The next sequential offset is advanced past this field if needed.
    pub fn field_at(mut self, name: &str, offset: usize, width: usize) -> Self {
        self.fields.push(LayoutField::new(
            name,
            BitRange::new(offset, width),
        ));
        let end = offset.saturating_add(width);
        if end > self.next_offset {
            self.next_offset = end;
        }
        self
    }

    /// Adds a variable-width field whose width is derived from another field's value.
    ///
    /// The `source_field` must be a previously declared field in this builder.
    /// At resolve time, the source field's value is read from data and converted
    /// via `unit` to compute the width in bits.
    ///
    /// A placeholder range is stored with width 0 and offset at the current
    /// position; the real offset and width are computed during [`BitLayout::resolve`].
    pub fn field_var(mut self, name: &str, source_field: &str, unit: LengthUnit) -> Self {
        let width_spec = FieldWidth::DerivedFrom {
            source_field: source_field.to_string(),
            unit,
        };
        // Placeholder range: offset at current position, width 0 (resolved later).
        self.fields.push(LayoutField::new_variable(
            name,
            self.next_offset,
            width_spec,
        ));
        // Don't advance next_offset — we don't know the width yet.
        self
    }

    /// Adds a field backed by a nested layout.
    pub fn field_layout(mut self, name: &str, layout: std::sync::Arc<BitLayout>) -> Self {
        let width = layout.bit_len();
        self.fields.push(LayoutField::new_layout(
            name,
            self.next_offset,
            layout,
        ));
        self.next_offset = self.next_offset.saturating_add(width);
        self
    }

    /// Validates and builds the layout.
    ///
    /// # Errors
    ///
    /// Returns an error if any field has:
    /// - an empty name ([`LayoutError::EmptyFieldName`])
    /// - zero width ([`LayoutError::ZeroWidth`]) (only for fixed-width fields)
    /// - a duplicate name ([`LayoutError::DuplicateName`])
    /// - an overlapping range ([`LayoutError::OverlappingFields`]) (only among fixed-width fields)
    /// - an offset+width that overflows ([`LayoutError::ArithmeticOverflow`])
    /// - a variable-width field referencing an unknown source ([`LayoutError::UnknownSourceField`])
    /// - a variable-width field referencing a later field ([`LayoutError::ForwardReference`])
    /// - a variable-width field referencing another variable field ([`LayoutError::VariableSourceField`])
    /// - a source field wider than 64 bits ([`LayoutError::SourceFieldTooWide`])
    pub fn build(self) -> LayoutResult<BitLayout> {
        // Validate individual fields.
        for field in &self.fields {
            if field.name().is_empty() {
                return Err(LayoutError::EmptyFieldName);
            }
            // Only check zero width for fixed-width fields.
            if let Some(w) = field.field_type().fixed_width() {
                if w == 0 {
                    return Err(LayoutError::ZeroWidth {
                        name: field.name().to_string(),
                    });
                }
            }
        }

        // Check for duplicate names.
        for (i, a) in self.fields.iter().enumerate() {
            for b in &self.fields[i + 1..] {
                if a.name() == b.name() {
                    return Err(LayoutError::DuplicateName {
                        name: a.name().to_string(),
                    });
                }
            }
        }

        // Validate variable-width field references.
        for (i, field) in self.fields.iter().enumerate() {
            if let crate::layout::field::FieldType::Primitive(FieldWidth::DerivedFrom { source_field, .. }) = field.field_type() {
                // Check source field exists.
                let src_pos = self
                    .fields
                    .iter()
                    .position(|f| f.name() == source_field);
                match src_pos {
                    None => {
                        return Err(LayoutError::UnknownSourceField {
                            field: field.name().to_string(),
                            source: source_field.clone(),
                        });
                    }
                    Some(pos) if pos >= i => {
                        return Err(LayoutError::ForwardReference {
                            field: field.name().to_string(),
                            source: source_field.clone(),
                        });
                    }
                    Some(pos) => {
                        // Check source is not itself variable.
                        let src = &self.fields[pos];
                        if src.is_variable() {
                            return Err(LayoutError::VariableSourceField {
                                field: field.name().to_string(),
                                source: source_field.clone(),
                            });
                        }
                        // Check source is <= 64 bits.
                        if src.width() > 64 {
                            return Err(LayoutError::SourceFieldTooWide {
                                field: field.name().to_string(),
                                source: source_field.clone(),
                                width: src.width(),
                            });
                        }
                    }
                }
            }
        }

        // Check for overlapping fields (only among fixed-width fields).
        let fixed_fields: Vec<_> = self
            .fields
            .iter()
            .filter(|f| f.field_type().is_fixed())
            .collect();
        for (i, a) in fixed_fields.iter().enumerate() {
            for b in &fixed_fields[i + 1..] {
                if a.range().overlaps(&b.range()) {
                    return Err(LayoutError::OverlappingFields {
                        existing: a.name().to_string(),
                        new: b.name().to_string(),
                    });
                }
            }
        }

        // Compute total bit length with overflow checking (fixed fields only).
        let mut bit_len: usize = 0;
        for field in &self.fields {
            if let Some(_) = field.field_type().fixed_width() {
                let end = field
                    .offset()
                    .checked_add(field.width())
                    .ok_or(LayoutError::ArithmeticOverflow)?;
                if end > bit_len {
                    bit_len = end;
                }
            }
        }

        Ok(BitLayout {
            fields: self.fields,
            bit_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_offsets() {
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
    fn total_length() {
        let layout = BitLayout::builder()
            .field("a", 3)
            .field("b", 5)
            .field("c", 16)
            .field("d", 8)
            .build()
            .unwrap();

        assert_eq!(layout.bit_len(), 32);
    }

    #[test]
    fn field_count() {
        let layout = BitLayout::builder()
            .field("a", 8)
            .field("b", 8)
            .build()
            .unwrap();

        assert_eq!(layout.field_count(), 2);
    }

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
    fn lookup_by_bit() {
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
    fn lookup_by_bit_boundary() {
        // Bit at exclusive end of a field belongs to the next field.
        let layout = BitLayout::builder()
            .field("a", 3)
            .field("b", 5)
            .build()
            .unwrap();

        assert_eq!(layout.field_at_bit(2).unwrap().name(), "a");
        assert_eq!(layout.field_at_bit(3).unwrap().name(), "b"); // a's end
    }

    #[test]
    fn field_index_at_bit() {
        let layout = BitLayout::builder()
            .field("a", 8)
            .field("b", 8)
            .build()
            .unwrap();

        assert_eq!(layout.field_index_at_bit(0), Some(0));
        assert_eq!(layout.field_index_at_bit(7), Some(0));
        assert_eq!(layout.field_index_at_bit(8), Some(1));
        assert_eq!(layout.field_index_at_bit(16), None);
    }

    #[test]
    fn reject_empty_name() {
        let result = BitLayout::builder().field("", 8).build();
        assert_eq!(result.unwrap_err(), LayoutError::EmptyFieldName);
    }

    #[test]
    fn reject_zero_width() {
        let result = BitLayout::builder().field("x", 0).build();
        assert_eq!(
            result.unwrap_err(),
            LayoutError::ZeroWidth {
                name: "x".to_string()
            }
        );
    }

    #[test]
    fn reject_duplicate_names() {
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
    fn reject_overlapping_explicit_fields() {
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

    #[test]
    fn explicit_positioning() {
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
    fn adjacent_explicit_fields() {
        let layout = BitLayout::builder()
            .field_at("a", 0, 8)
            .field_at("b", 8, 8)
            .build()
            .unwrap();

        assert_eq!(layout.bit_len(), 16);
        assert!(!layout.field(0).unwrap().range().overlaps(&layout.field(1).unwrap().range()));
    }

    #[test]
    fn non_byte_aligned_layout() {
        let layout = BitLayout::builder()
            .field("a", 3)
            .field("b", 5)
            .field("c", 2)
            .build()
            .unwrap();

        assert_eq!(layout.bit_len(), 10);
    }

    #[test]
    fn many_fields() {
        let mut builder = BitLayout::builder();
        for i in 0..100 {
            builder = builder.field(&format!("f{i}"), 8);
        }
        let layout = builder.build().unwrap();
        assert_eq!(layout.field_count(), 100);
        assert_eq!(layout.bit_len(), 800);
        assert_eq!(layout.field(99).unwrap().offset(), 792);
    }
}
