use super::error::{LayoutError, LayoutResult};
use super::field::FieldWidth;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    name: String,
    width: FieldWidth,
    by_value: HashMap<u64, String>,
    by_name: HashMap<String, u64>,
}

impl EnumDef {
    pub fn new(
        name: impl Into<String>,
        width: FieldWidth,
        variants: Vec<EnumVariant>,
    ) -> LayoutResult<Self> {
        let name = name.into();

        let mut by_value = HashMap::new();
        let mut by_name = HashMap::new();
        let max_val = if let FieldWidth::Fixed(w) = width {
            if w < 64 {
                (1u64 << w).wrapping_sub(1)
            } else {
                u64::MAX
            }
        } else {
            u64::MAX // If variable width, we can't strictly bound it at definition time easily unless we add logic, but standard max is 64.
        };

        for v in variants {
            if v.value > max_val {
                return Err(LayoutError::ArithmeticOverflow); // Replace with better error if needed
            }
            if by_name.contains_key(&v.name) {
                return Err(LayoutError::DuplicateName { name: v.name });
            }
            if by_value.contains_key(&v.value) {
                return Err(LayoutError::DuplicateName {
                    name: format!("value {}", v.value),
                });
            }
            by_name.insert(v.name.clone(), v.value);
            by_value.insert(v.value, v.name);
        }

        Ok(Self {
            name,
            width,
            by_value,
            by_name,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn width(&self) -> &FieldWidth {
        &self.width
    }

    pub fn variant_name(&self, value: u64) -> Option<&str> {
        self.by_value.get(&value).map(|s| s.as_str())
    }

    pub fn variant_value(&self, name: &str) -> Option<u64> {
        self.by_name.get(name).copied()
    }
}

impl EnumDef {
    pub fn with_width(&self, width: FieldWidth) -> Self {
        Self {
            name: self.name.clone(),
            width,
            by_value: self.by_value.clone(),
            by_name: self.by_name.clone(),
        }
    }
}
