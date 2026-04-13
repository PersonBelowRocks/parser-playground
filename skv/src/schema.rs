use std::collections::{HashMap, HashSet};

use skv_core::{EnumString, SkvEnum};

use crate::{Key, Value, ValueType, util::Sealed};

#[derive(Clone, Default)]
pub struct Schema {
    pub name: Option<String>,
    pub values: HashMap<Key, SchemaValue>,
    pub unknown_keys: UnknownKeyBehaviour,
}

impl Schema {
    #[inline]
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
            values: HashMap::new(),
            unknown_keys: UnknownKeyBehaviour::default(),
        }
    }

    #[inline]
    pub fn unknown_key_behaviour(&mut self, behaviour: UnknownKeyBehaviour) -> &mut Self {
        self.unknown_keys = behaviour;
        self
    }

    #[inline]
    pub fn name(&mut self, name: impl ToString) -> &mut Self {
        self.name = Some(name.to_string());
        self
    }

    #[inline]
    pub fn value(&mut self, key: Key, val: impl Into<SchemaValue>) -> &mut Self {
        self.values.insert(key, val.into());
        self
    }

    #[inline]
    pub fn get_value(&self, key: &Key) -> Option<&SchemaValue> {
        self.values.get(key)
    }

    pub(crate) fn enums(&self) -> HashMap<Key, EnumExpectations> {
        let enums = self.values.iter().filter_map(|(key, val)| match val {
            SchemaValue::Enum(ex) => Some((key.clone(), ex.clone())),
            _ => None,
        });

        HashMap::from_iter(enums)
    }
}

/// How a schema handles unknown keys when parsing (i.e. keys not in the schema).
///
/// The default behaviour is to parse the keys: [`UnknownKeyBehaviour::Parse`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum UnknownKeyBehaviour {
    /// Skip the keys, don't include them and their values in the output.
    Skip,
    /// Parse the keys, inferring their types, and include them in the output.
    #[default]
    Parse,
    /// Fail when unknown keys are encountered.
    Error,
}

#[derive(Debug, Clone, derive_more::From)]
pub enum SchemaValue {
    String(ValueExpectations<String>),
    Enum(EnumExpectations),
    Int(ValueExpectations<i64>),
    Double(ValueExpectations<f64>),
    Bool(ValueExpectations<bool>),
}

macro_rules! impl_sch_val_from_behaviour {
    ($input:ty, $variant:path) => {
        impl From<ValueBehaviour<$input>> for SchemaValue {
            #[inline]
            fn from(behaviour: ValueBehaviour<$input>) -> Self {
                $variant(ValueExpectations::from(behaviour))
            }
        }
    };
}

impl_sch_val_from_behaviour!(String, SchemaValue::String);
impl_sch_val_from_behaviour!(i64, SchemaValue::Int);
impl_sch_val_from_behaviour!(f64, SchemaValue::Double);
impl_sch_val_from_behaviour!(bool, SchemaValue::Bool);

impl SchemaValue {
    #[inline]
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::String(_) => ValueType::String,
            Self::Enum(_) => ValueType::Enum,
            Self::Int(_) => ValueType::Int,
            Self::Double(_) => ValueType::Double,
            Self::Bool(_) => ValueType::Bool,
        }
    }

    #[inline]
    pub fn behaviour(self) -> ValueBehaviour<Value> {
        match self.clone() {
            Self::Bool(ex) => ex.behaviour.map_default(Value::Bool),
            Self::Int(ex) => ex.behaviour.map_default(Value::Int),
            Self::Double(ex) => ex.behaviour.map_default(Value::Double),
            Self::String(ex) => ex.behaviour.map_default(Value::String),
            Self::Enum(ex) => match ex.default {
                Some(default) => ValueBehaviour::Default(Value::Enum(default)),
                None => ValueBehaviour::Required,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumExpectations {
    default: Option<EnumString>,
    values: HashSet<EnumString>,
}

impl EnumExpectations {
    #[inline]
    pub fn contains(&self, enum_string: &EnumString) -> bool {
        self.values.contains(enum_string)
    }

    #[inline]
    pub fn get_default(&self) -> Option<&EnumString> {
        self.default.as_ref()
    }

    #[inline]
    pub fn from_enum<T: SkvEnum>() -> Self {
        Self {
            default: None,
            values: T::enum_strings(),
        }
    }

    #[inline]
    pub fn from_enum_default<T: SkvEnum>(default: T) -> Self {
        Self {
            default: Some(default.to_enum_string()),
            values: T::enum_strings(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValueExpectations<T: BaseType> {
    pub suggestions: HashSet<T>,
    pub behaviour: ValueBehaviour<T>,
}

impl<T: BaseType> Default for ValueExpectations<T> {
    fn default() -> Self {
        Self {
            suggestions: HashSet::default(),
            behaviour: ValueBehaviour::default(),
        }
    }
}

impl<T: BaseType> From<ValueBehaviour<T>> for ValueExpectations<T> {
    #[inline]
    fn from(behaviour: ValueBehaviour<T>) -> Self {
        Self {
            behaviour,
            ..Default::default()
        }
    }
}

#[derive(Clone, Default, Debug)]
pub enum ValueBehaviour<T> {
    #[default]
    Required,
    Optional,
    Default(T),
}

impl<T> ValueBehaviour<T> {
    /// Maps the default value.
    #[inline]
    pub fn map_default<O>(self, f: impl FnOnce(T) -> O) -> ValueBehaviour<O> {
        match self {
            ValueBehaviour::Required => ValueBehaviour::Required,
            ValueBehaviour::Optional => ValueBehaviour::Optional,
            ValueBehaviour::Default(t) => ValueBehaviour::Default(f(t)),
        }
    }
}

#[allow(private_bounds)]
pub trait BaseType: Sealed + Sized {
    const VALUE_TYPE: ValueType;

    fn ref_from_value(val: &Value) -> Option<&Self>;
    fn mut_from_value(val: &mut Value) -> Option<&mut Self>;
    fn from_value(val: Value) -> Option<Self>;
}

macro_rules! impl_base_type {
    ($t:ty, $value_type:expr, $p:path) => {
        impl crate::util::Sealed for $t {}
        impl BaseType for $t {
            const VALUE_TYPE: ValueType = $value_type;

            #[inline]
            fn ref_from_value(val: &Value) -> Option<&Self> {
                match val {
                    $p(inner) => Some(inner),
                    _ => None,
                }
            }

            #[inline]
            fn mut_from_value(val: &mut Value) -> Option<&mut Self> {
                match val {
                    $p(inner) => Some(inner),
                    _ => None,
                }
            }

            #[inline]
            fn from_value(val: Value) -> Option<Self> {
                match val {
                    $p(inner) => Some(inner),
                    _ => None,
                }
            }
        }
    };
}

impl_base_type!(String, ValueType::String, Value::String);
impl_base_type!(EnumString, ValueType::Enum, Value::Enum);
impl_base_type!(i64, ValueType::Int, Value::Int);
impl_base_type!(f64, ValueType::Double, Value::Double);
impl_base_type!(bool, ValueType::Bool, Value::Bool);
