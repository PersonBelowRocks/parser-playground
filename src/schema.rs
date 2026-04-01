use std::{
    collections::{HashMap, HashSet},
    ops::{Bound, RangeBounds},
};

use crate::{Key, ValueType};

#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub schema_name: Option<String>,
    pub values: HashMap<Key, SchemaValue>,
    pub skip_unknown_keys: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaValue {
    pub optional: bool,
    pub value_type: SchemaValueType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaValueType {
    Basic(ValueType),
    Constrained(ConstraintType),
}

impl SchemaValueType {
    /// Returns the [`ValueType`] this schema value accepts.
    #[inline]
    pub fn value_type(&self) -> ValueType {
        match self {
            SchemaValueType::Basic(t) => *t,
            SchemaValueType::Constrained(c) => c.value_type(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintType {
    String(StringConstraint),
    Int(IntConstraint),
    Double(RangeConstraint<f64>),
}

impl ConstraintType {
    /// Returns the underlying [`ValueType`] this constraint applies to.
    #[inline]
    pub fn value_type(&self) -> ValueType {
        match self {
            ConstraintType::String(_) => ValueType::String,
            ConstraintType::Int(_) => ValueType::Int,
            ConstraintType::Double(_) => ValueType::Double,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringConstraint {
    Enum(HashSet<String>),
    Regex(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntConstraint {
    Range(RangeConstraint<i64>),
    Enum(HashSet<i64>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeConstraint<T> {
    pub from: Bound<T>,
    pub to: Bound<T>,
}

macro_rules! impl_range_constraint_from_bounds {
    ($t:ty) => {
        impl<R: RangeBounds<$t>> From<R> for RangeConstraint<$t> {
            fn from(range: R) -> Self {
                Self {
                    from: range.start_bound().cloned(),
                    to: range.end_bound().cloned(),
                }
            }
        }
    };
}

impl_range_constraint_from_bounds!(i64);
impl_range_constraint_from_bounds!(f64);
