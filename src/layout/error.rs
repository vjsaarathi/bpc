//! Error types for layout operations.

use std::fmt;

/// Errors that can occur during layout construction or validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    /// Field name is empty.
    EmptyFieldName,

    /// Field name contains invalid characters (e.g. '.').
    InvalidFieldName {
        /// Name of the invalid field.
        name: String,
    },

    /// Field width is zero.
    ZeroWidth {
        /// Name of the zero-width field.
        name: String,
    },

    /// Layout contains duplicate field names.
    DuplicateName {
        /// The duplicated name.
        name: String,
    },

    /// Two fields overlap in their bit ranges.
    OverlappingFields {
        /// Name of the existing field.
        existing: String,
        /// Name of the conflicting new field.
        new: String,
    },

    /// Arithmetic overflow when computing offsets or total length.
    ArithmeticOverflow,

    /// A variable-width field references a source field that does not exist.
    UnknownSourceField {
        /// The field with the derived width.
        field: String,
        /// The missing source field name.
        source: String,
    },

    /// A variable-width field references a source field declared after it.
    ForwardReference {
        /// The field with the derived width.
        field: String,
        /// The source field that appears later.
        source: String,
    },

    /// The source field for a derived width is itself variable-width.
    VariableSourceField {
        /// The field with the derived width.
        field: String,
        /// The source field that is also variable.
        source: String,
    },

    /// The source field is too wide (> 64 bits) to read as a length value.
    SourceFieldTooWide {
        /// The field with the derived width.
        field: String,
        /// The source field that is too wide.
        source: String,
        /// Width of the source field in bits.
        width: usize,
    },

    /// Not enough data to resolve a variable-width field.
    InsufficientData {
        /// The field that could not be resolved.
        field: String,
        /// Bits needed to read the source field.
        needed_bits: usize,
        /// Bits available in the data.
        available_bits: usize,
    },

    /// A resolved variable-width field has zero width.
    ResolvedZeroWidth {
        /// The field that resolved to zero width.
        field: String,
    },
}

/// Convenience alias for layout operation results.
pub type LayoutResult<T> = Result<T, LayoutError>;

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFieldName => write!(f, "field name must not be empty"),
            Self::InvalidFieldName { name } => write!(f, "field name \"{name}\" is invalid (cannot contain '.')"),
            Self::ZeroWidth { name } => write!(f, "field \"{name}\" has zero width"),
            Self::DuplicateName { name } => write!(f, "duplicate field name \"{name}\""),
            Self::OverlappingFields { existing, new } => {
                write!(f, "field \"{new}\" overlaps with \"{existing}\"")
            }
            Self::ArithmeticOverflow => write!(f, "layout size overflow"),
            Self::UnknownSourceField { field, source } => {
                write!(f, "field \"{field}\" references unknown source field \"{source}\"")
            }
            Self::ForwardReference { field, source } => {
                write!(
                    f,
                    "field \"{field}\" references source field \"{source}\" which is declared after it"
                )
            }
            Self::VariableSourceField { field, source } => {
                write!(
                    f,
                    "field \"{field}\" references source field \"{source}\" which is itself variable-width"
                )
            }
            Self::SourceFieldTooWide {
                field,
                source,
                width,
            } => {
                write!(
                    f,
                    "field \"{field}\": source field \"{source}\" is {width} bits wide (max 64)"
                )
            }
            Self::InsufficientData {
                field,
                needed_bits,
                available_bits,
            } => {
                write!(
                    f,
                    "cannot resolve field \"{field}\": need {needed_bits} bits but only {available_bits} available"
                )
            }
            Self::ResolvedZeroWidth { field } => {
                write!(f, "field \"{field}\" resolved to zero width")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_empty_name() {
        assert_eq!(LayoutError::EmptyFieldName.to_string(), "field name must not be empty");
    }

    #[test]
    fn display_zero_width() {
        let e = LayoutError::ZeroWidth { name: "x".into() };
        assert_eq!(e.to_string(), "field \"x\" has zero width");
    }

    #[test]
    fn display_duplicate() {
        let e = LayoutError::DuplicateName { name: "id".into() };
        assert_eq!(e.to_string(), "duplicate field name \"id\"");
    }

    #[test]
    fn display_overlap() {
        let e = LayoutError::OverlappingFields {
            existing: "a".into(),
            new: "b".into(),
        };
        assert_eq!(e.to_string(), "field \"b\" overlaps with \"a\"");
    }

    #[test]
    fn error_is_clone_and_eq() {
        let e1 = LayoutError::ArithmeticOverflow;
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }
}
