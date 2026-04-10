use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
};

use strum::IntoDiscriminant;
use winnow::{
    ascii,
    combinator::{cut_err, separated},
    prelude::*,
};

use crate::{
    Key, KeyValuePair, Value, ValueType,
    error::{ErrorFromParts, MapError, MapParseError},
    key_value_pair::skv_pair,
    schema::{BaseType, Schema, ValueBehaviour},
};

pub(crate) type InnerMap<H = RandomState> = HashMap<Key, Value, H>;

#[derive(Clone, Debug, derive_more::Into)]
pub struct ParsedMap<H = RandomState>(InnerMap<H>);

impl ParsedMap {
    #[inline]
    pub fn parse(schema: &Schema, input: impl AsRef<str>) -> Result<Self, MapParseError> {
        Self::parse_with_hasher(RandomState::default(), schema, input)
    }
}

impl PartialEq for ParsedMap {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<H: BuildHasher> ParsedMap<H> {
    #[inline]
    pub fn parse_with_hasher(
        hasher: H,
        schema: &Schema,
        input: impl AsRef<str>,
    ) -> Result<Self, MapParseError>
    where
        H: Default,
    {
        parse_map_with_hasher(hasher, Some(schema), input).map(Self)
    }

    #[inline]
    pub fn get<T: BaseType>(&self, key: &Key) -> Result<&T, MapError> {
        let val = self.0.get(key).ok_or(MapError::NotFound)?;
        val.get_ref::<T>().ok_or_else(|| MapError::WrongType {
            expected: T::VALUE_TYPE,
            found: val.discriminant(),
        })
    }

    #[inline]
    pub fn get_mut<T: BaseType>(&mut self, key: &Key) -> Result<&mut T, MapError> {
        let val = self.0.get_mut(key).ok_or(MapError::NotFound)?;
        let found = val.discriminant();
        val.get_mut::<T>().ok_or(MapError::WrongType {
            expected: T::VALUE_TYPE,
            found,
        })
    }

    #[inline]
    pub fn get_int(&self, key: &Key) -> Result<i64, MapError> {
        self.get::<i64>(key).cloned()
    }

    #[inline]
    pub fn get_double(&self, key: &Key) -> Result<f64, MapError> {
        self.get::<f64>(key).cloned()
    }

    #[inline]
    pub fn get_bool(&self, key: &Key) -> Result<bool, MapError> {
        self.get::<bool>(key).cloned()
    }

    #[inline]
    pub fn get_str(&self, key: &Key) -> Result<&str, MapError> {
        self.get::<String>(key).map(String::as_str)
    }
}

/// Parse an SKV map with a custom hasher.
#[inline]
#[allow(unused)]
pub(crate) fn parse_map_with_hasher<H: BuildHasher>(
    builder: H,
    schema: Option<&Schema>,
    input: impl AsRef<str>,
) -> Result<InnerMap<H>, MapParseError> {
    separated(0.., cut_err(skv_pair(schema)), ascii::multispace1)
        .parse(input.as_ref().trim())
        .map_err(MapParseError::from_parse_error)
        .and_then(|pairs: Vec<KeyValuePair>| {
            // keys that the schema requires be present
            let mut required_keys = HashMap::<Key, ValueType>::new();

            let mut map = InnerMap::<H>::with_capacity_and_hasher(pairs.len(), builder);

            if let Some(schema) = schema {
                for (key, value) in schema.values.iter() {
                    match value.clone().behaviour() {
                        ValueBehaviour::Required => {
                            required_keys.insert(key.clone(), value.value_type());
                        }
                        ValueBehaviour::Default(val) => {
                            map.insert(key.clone(), val);
                        }
                        _ => {}
                    };
                }
            }

            for pair in pairs.into_iter() {
                if let Some((key, value_type)) = required_keys.remove_entry(&pair.key) {
                    // this should not be possible, but it's awfully convenient to do a check here
                    assert_eq!(
                        value_type,
                        pair.value.discriminant(),
                        "required key '{}' has value of type {:?} but parsed to type {:?}",
                        key.as_ref(),
                        value_type,
                        pair.value.discriminant()
                    );
                }

                map.insert(pair.key, pair.value);
            }

            if !required_keys.is_empty() {
                Err(MapParseError::MissingRequiredKeys(required_keys))
            } else {
                Ok(map)
            }
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::error::MapParseError;
    use crate::util::testing::key;
    use crate::{Schema, ValueBehaviour, ValueType};

    use super::ParsedMap;

    mod proptests {
        use super::*;
        use crate::{Key, SchemaValue, Value};
        use proptest::prelude::*;
        use strum::IntoDiscriminant;

        #[derive(Debug)]
        struct Case {
            schema_val: SchemaValue,
            key: Key,
            kv_pair_string: String,
            expected: Value,
        }

        #[derive(Debug, Clone, Copy)]
        enum Radix {
            Binary,
            Octal,
            Decimal,
            Hexadecimal,
        }

        fn radix_strategy() -> impl Strategy<Value = Radix> {
            prop_oneof![
                Just(Radix::Binary),
                Just(Radix::Octal),
                Just(Radix::Decimal),
                Just(Radix::Hexadecimal),
            ]
        }

        fn key_strategy() -> impl Strategy<Value = Key> {
            proptest::collection::vec("[a-z_]+[a-z0-9_]*", 1..8)
                .prop_flat_map(|parts| Just(key(parts.join(".").to_string())))
        }

        fn val_strategy() -> impl Strategy<Value = Value> {
            // TODO: generate values and string representations at the same time
            prop_oneof![
                any::<i64>().prop_map(Value::Int),
                any::<f64>().prop_map(Value::Double),
                any::<bool>().prop_map(Value::Bool),
                r#"[^\p{C}'"\\]"#.prop_map(|s| { s }).prop_map(Value::String),
            ]
        }

        fn sch_val_behaviour_strategy(val: Value) -> impl Strategy<Value = ValueBehaviour<Value>> {
            prop_oneof![
                Just(ValueBehaviour::Optional),
                Just(ValueBehaviour::Default(val)),
                Just(ValueBehaviour::Required),
            ]
        }

        fn value_str_strategy(val: &Value) -> BoxedStrategy<String> {
            match val {
                Value::Int(i) => {
                    // we do this dance around negativity since rust's non-decimal formatting doesn't add the sign
                    // and instead interprets the number as unsigned
                    let is_negative = i.is_negative();
                    let abs = i.abs();

                    radix_strategy()
                        .prop_map(move |radix| {
                            let formatted = match radix {
                                Radix::Decimal => format!("{abs}"),
                                Radix::Binary => format!("{abs:#b}"),
                                Radix::Octal => format!("{abs:#o}"),
                                Radix::Hexadecimal => format!("{abs:#x}"),
                            };

                            if is_negative {
                                format!("-{formatted}")
                            } else {
                                formatted
                            }
                        })
                        .boxed()
                }
                Value::Double(f) => Just(format!("{f}")).boxed(),
                Value::Bool(b) => match *b {
                    true => "[Tt][Rr][Uu][Ee]",
                    false => "[Ff][Aa][Ll][Ss][Ee]",
                }
                .boxed(),
                Value::String(s) => {
                    if s.contains(|c: char| c.is_whitespace()) || s.is_empty() {
                        Just(format!("\"{}\"", s)).boxed()
                    } else {
                        Just(s.clone()).boxed()
                    }
                }
            }
        }

        prop_compose! {
            fn case_strategy()(val in val_strategy())(
                key in key_strategy(),
                val_type in Just(val.discriminant()),
                val in Just(val.clone()),
                behaviour in sch_val_behaviour_strategy(val.clone()),
                value_str in value_str_strategy(&val),
                pad_eq_l in any::<bool>(),
                pad_eq_r in any::<bool>(),
            ) -> Case {
                let schema_val = match val_type {
                    ValueType::Int => SchemaValue::Int(behaviour.map_default(Value::get::<i64>).map_default(Option::unwrap).into()),
                    ValueType::Double => SchemaValue::Double(behaviour.map_default(Value::get::<f64>).map_default(Option::unwrap).into()),
                    ValueType::Bool => SchemaValue::Bool(behaviour.map_default(Value::get::<bool>).map_default(Option::unwrap).into()),
                    ValueType::String => SchemaValue::String(behaviour.map_default(Value::get::<String>).map_default(Option::unwrap).into()),
                };

                let kv_pair_string = format!(
                    "{}{}={}{}",
                    key.as_ref(),
                    if pad_eq_l { " " } else { "" },
                    if pad_eq_r { " " } else { "" },
                    value_str,
                );

                Case {
                    schema_val,
                    key,
                    kv_pair_string,
                    expected: val,
                }
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(1000))]
            #[test]
            fn schemaed_map(cases in prop::collection::vec(case_strategy(), 1..10)) {
                let mut schema = Schema::default();
                for case in cases.iter() {
                    schema.value(case.key.clone(), case.schema_val.clone());
                }

                let input = cases.iter().map(|case| &case.kv_pair_string).cloned().collect::<Vec<_>>().join(" ");
                let map = match ParsedMap::parse(&schema, input) {
                    Ok(map) => map,
                    Err(e) => panic!("{:?}", miette::Report::new(e)),
                };

                for case in cases.iter() {
                    match &case.expected {
                        Value::Int(ex) => assert_eq!(map.get_int(&case.key), Ok(*ex)),
                        Value::Double(ex) => assert_eq!(map.get_double(&case.key), Ok(*ex)),
                        Value::Bool(ex) => assert_eq!(map.get_bool(&case.key), Ok(*ex)),
                        Value::String(ex) => assert_eq!(map.get_str(&case.key), Ok(ex.as_str())),
                    }
                }
            }
        }
    }

    #[test]
    fn required_value() {
        let mut schema = Schema::default();
        schema.value(key("key.test"), ValueBehaviour::<i64>::Required);

        let map = ParsedMap::parse(&schema, "key.test=42").unwrap();
        assert_eq!(map.get_int(&key("key.test")).unwrap(), 42);

        // missing key
        assert_eq!(
            ParsedMap::parse(&schema, "not.key.test=42"),
            Err(MapParseError::MissingRequiredKeys(HashMap::from([(
                key("key.test"),
                ValueType::Int
            )])))
        );
        // wrong type
        assert!(ParsedMap::parse(&schema, "key.test=42.0").is_err());
    }

    #[test]
    fn default_value() {
        let mut schema = Schema::default();
        schema.value(key("key.test"), ValueBehaviour::<i64>::Default(42));

        let map = ParsedMap::parse(&schema, "something_else=13").unwrap();
        assert_eq!(map.get_int(&key("key.test")).unwrap(), 42);
        assert_eq!(map.get_int(&key("something_else")).unwrap(), 13);

        let map = ParsedMap::parse(&schema, "something_else=13 key.test=24").unwrap();
        assert_eq!(map.get_int(&key("key.test")).unwrap(), 24);
        assert_eq!(map.get_int(&key("something_else")).unwrap(), 13);

        // wrong type
        assert!(ParsedMap::parse(&schema, "key.test=true").is_err());
    }

    #[test]
    fn optional_value() {
        let mut schema = Schema::default();
        schema.value(key("key.test"), ValueBehaviour::<i64>::Optional);

        let map = ParsedMap::parse(&schema, "something_else=13").unwrap();
        assert!(map.get_int(&key("key.test")).is_err());
        assert_eq!(map.get_int(&key("something_else")).unwrap(), 13);

        let map = ParsedMap::parse(&schema, "something_else=13 key.test=24").unwrap();
        assert_eq!(map.get_int(&key("key.test")).unwrap(), 24);
        assert_eq!(map.get_int(&key("something_else")).unwrap(), 13);

        // wrong type
        assert!(ParsedMap::parse(&schema, "key.test=true").is_err());
    }
}
