//! Bit layout representation and field positioning.
//!
//! Provides types for describing sequences of named bit fields with calculated
//! positions, validating field constraints, and looking up fields by position.
//!
//! # Overview
//!
//! - [`BitRange`] — a contiguous half-open range of bits `[offset, end)`.
//! - [`LayoutField`] — a named bit region (name + range).
//! - [`BitLayout`] — an ordered collection of fields with validation.
//! - [`BitLayoutBuilder`] — builder for constructing layouts.
//!
//! # Examples
//!
//! ```
//! use bpc::layout::BitLayout;
//!
//! let layout = BitLayout::builder()
//!     .field("version", 3)
//!     .field("opcode", 5)
//!     .field("length", 16)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(layout.bit_len(), 24);
//! assert_eq!(layout.field_at_bit(0).unwrap().name(), "version");
//! assert_eq!(layout.field_at_bit(3).unwrap().name(), "opcode");
//! ```

pub mod error;
pub mod field;
pub mod layout;

pub use error::{LayoutError, LayoutResult};
pub use field::{BitRange, FieldWidth, LayoutField, LengthUnit};
pub use layout::{BitLayout, BitLayoutBuilder};
pub mod enum_def;
