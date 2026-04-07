use std::{collections::{HashMap, HashSet}};

use crate::{Key, ValueType, util::Sealed};

#[derive(Clone)]
pub struct Schema {
    pub schema_name: Option<String>,
    pub values: HashMap<Key, SchemaValue>,
    pub skip_unknown_keys: bool,
}

#[derive(Clone)]
pub enum SchemaValue {
    String(ValueExpectations<String>),
    Int(ValueExpectations<i64>),
    Double(ValueExpectations<f64>),
    Bool(ValueExpectations<bool>),
}

impl SchemaValue {
    #[inline]
    pub fn value_type(&self) -> ValueType {
        match self {
            SchemaValue::String(_) => ValueType::String,
            SchemaValue::Int(_) => ValueType::Int,
            SchemaValue::Double(_) => ValueType::Double,
            SchemaValue::Bool(_) => ValueType::Bool,
        }
    }
}

#[derive(Clone)]
pub struct ValueExpectations<T: BaseType> {
    pub suggestions: HashSet<T>,
    pub behaviour: SchemaValueBehaviour<T>,
}

#[derive(Clone)]
pub enum SchemaValueBehaviour<T: BaseType> {
    Required,
    Optional,
    Default(T)
}

pub trait BaseType: Sealed {
    const VALUE_TYPE: ValueType;
}

macro_rules! impl_base_type {
    ($t:ty, $value_type:expr) => {
        impl crate::util::Sealed for $t {}
        impl BaseType for $t {
            const VALUE_TYPE: ValueType = $value_type;
        }
    };
}


impl_base_type!(String, ValueType::String);
impl_base_type!(i64, ValueType::Int);
impl_base_type!(f64, ValueType::Double);
impl_base_type!(bool, ValueType::Bool);
