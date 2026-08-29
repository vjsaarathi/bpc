//! Error types for bit-level operations.

use std::fmt;

/// Errors that can occur during bit-level read/write operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitError {
    /// Attempted to read more bits than are available in the buffer.
    UnexpectedEof {
        /// Current bit position when the error occurred.
        position: usize,
        /// Number of bits that were requested.
        requested: usize,
        /// Number of bits actually remaining.
        remaining: usize,
    },

    /// Value does not fit in the specified number of bits.
    ValueOverflow {
        /// The value that was too large.
        value: u128,
        /// The number of bits available to hold the value.
        bits: u32,
    },

    /// Requested bit count exceeds the maximum allowed for the operation.
    InvalidBitCount {
        /// The requested bit count.
        bits: u32,
        /// The maximum allowed for this operation.
        max: u32,
    },
}

/// Convenience alias for results of bit operations.
pub type BitResult<T> = Result<T, BitError>;

impl fmt::Display for BitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof {
                position,
                requested,
                remaining,
            } => {
                write!(
                    f,
                    "unexpected end of data at bit {position}: \
                     requested {requested} bits, {remaining} remaining"
                )
            }
            Self::ValueOverflow { value, bits } => {
                write!(f, "value {value} does not fit in {bits} bits")
            }
            Self::InvalidBitCount { bits, max } => {
                write!(f, "bit count {bits} exceeds maximum of {max}")
            }
        }
    }
}

impl std::error::Error for BitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unexpected_eof() {
        let err = BitError::UnexpectedEof {
            position: 5,
            requested: 8,
            remaining: 3,
        };
        assert_eq!(
            err.to_string(),
            "unexpected end of data at bit 5: requested 8 bits, 3 remaining"
        );
    }

    #[test]
    fn display_value_overflow() {
        let err = BitError::ValueOverflow {
            value: 255,
            bits: 7,
        };
        assert_eq!(err.to_string(), "value 255 does not fit in 7 bits");
    }

    #[test]
    fn display_invalid_bit_count() {
        let err = BitError::InvalidBitCount { bits: 65, max: 64 };
        assert_eq!(err.to_string(), "bit count 65 exceeds maximum of 64");
    }

    #[test]
    fn error_is_clone_and_eq() {
        let err1 = BitError::UnexpectedEof {
            position: 0,
            requested: 1,
            remaining: 0,
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
