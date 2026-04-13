use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
};

use skv_core::SkvEnum;
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
    schema::{BaseType, EnumExpectations, Schema, ValueBehaviour},
};

pub(crate) type InnerMap<H = RandomState> = HashMap<Key, Value, H>;

#[derive(Clone, Debug)]
pub struct ParsedMap<H = RandomState> {
    map: InnerMap<H>,
    enums: HashMap<Key, EnumExpectations>,
}

impl ParsedMap {
    #[inline]
    pub fn parse(schema: &Schema, input: impl AsRef<str>) -> Result<Self, MapParseError> {
        Self::parse_with_hasher(RandomState::default(), schema, input)
    }
}

impl PartialEq for ParsedMap {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
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
        Ok(Self {
            map: parse_map_with_hasher(hasher, Some(schema), input)?,
            enums: schema.enums(),
        })
    }

    #[inline]
    pub fn get<T: BaseType>(&self, key: &Key) -> Result<&T, MapError> {
        let val = self.map.get(key).ok_or(MapError::NotFound)?;
        val.get_ref::<T>().ok_or_else(|| MapError::WrongType {
            expected: T::VALUE_TYPE,
            found: val.discriminant(),
        })
    }

    #[inline]
    pub fn get_mut<T: BaseType>(&mut self, key: &Key) -> Result<&mut T, MapError> {
        let val = self.map.get_mut(key).ok_or(MapError::NotFound)?;
        let found = val.discriminant();
        val.get_mut::<T>().ok_or(MapError::WrongType {
            expected: T::VALUE_TYPE,
            found,
        })
    }

    #[inline]
    pub fn get_value(&self, key: &Key) -> Option<&Value> {
        self.map.get(key)
    }

    #[inline]
    pub fn get_enum<T: SkvEnum>(&self, key: &Key) -> Result<T, MapError> {
        let val = self
            .map
            .get(key)
            .ok_or(MapError::NotFound)
            .and_then(|val| match val {
                Value::Enum(enum_string) => Ok(enum_string),
                other => Err(MapError::WrongType {
                    expected: ValueType::Enum,
                    found: other.discriminant(),
                }),
            })?;

        T::from_enum_string(val).ok_or(MapError::WrongEnum)
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
    use skv_core::{EnumString, SkvEnum};
    use skv_macros::SkvEnum;
    use std::collections::{HashMap, HashSet};
    use strum::VariantArray;

    use crate::error::MapParseError;
    use crate::schema::EnumExpectations;
    use crate::util::testing::key;
    use crate::{Schema, ValueBehaviour, ValueType};

    use super::ParsedMap;

    mod proptests {
        use super::*;
        use crate::{Key, SchemaValue, Value};
        use proptest::prelude::*;
        use strum::IntoDiscriminant;

        /// A key-value pair test case. Multiple of these are combined into one map which is then parsed and tested.
        #[derive(Debug, Clone)]
        struct Case {
            schema_val: SchemaValue,
            key: Key,
            kv_pair_string: String,
            expected: Value,
        }

        /// A value and the string representation that should parse to it.
        #[derive(Debug, Clone)]
        struct ExpectedValue {
            expected: Value,
            serialized: String,
        }

        #[derive(Debug, Clone, Copy)]
        enum Radix {
            Binary,
            Octal,
            Decimal,
            Hexadecimal,
        }

        impl Radix {
            fn format(&self, val: i64) -> String {
                // we do this dance around negativity since rust's non-decimal formatting doesn't add the sign
                // and instead interprets the number as unsigned
                let is_negative = val.is_negative();
                let abs = val.abs();

                let formatted = match self {
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
            }
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

        fn int_strategy() -> impl Strategy<Value = ExpectedValue> {
            (radix_strategy(), any::<i64>()).prop_map(|(radix, val)| ExpectedValue {
                expected: Value::Int(val),
                serialized: radix.format(val),
            })
        }

        fn double_strategy() -> impl Strategy<Value = ExpectedValue> {
            any::<f64>().prop_map(|v| ExpectedValue {
                expected: Value::Double(v),
                serialized: format!("{v}"),
            })
        }

        fn bool_strategy() -> impl Strategy<Value = ExpectedValue> {
            any::<bool>()
                .prop_flat_map(|v| {
                    (
                        Just(v),
                        match v {
                            true => "[Tt][Rr][Uu][Ee]",
                            false => "[Ff][Aa][Ll][Ss][Ee]",
                        },
                    )
                })
                .prop_map(|(val, string)| ExpectedValue {
                    expected: Value::Bool(val),
                    serialized: string.to_string(),
                })
        }

        fn string_strategy() -> impl Strategy<Value = ExpectedValue> {
            (
                r#"\p{C}"#.prop_filter("string can't contain null char", |s| !s.contains('\0')),
                any::<bool>(),
            )
                .prop_map(|(expected_string, double_quotes)| {
                    let quote = if double_quotes { '"' } else { '\'' };
                    let opposite_quote = if double_quotes { '\'' } else { '"' };
                    // string should be quoted if it contains quotes or whitespace, or if it's empty
                    let mut should_quote = expected_string.is_empty();

                    // we add a little extra capacity here to work with
                    let mut serialized_string = String::with_capacity(expected_string.len() + 8);
                    for ch in expected_string.chars() {
                        match ch {
                            '\n' => serialized_string.push_str("\\n"),
                            '\r' => serialized_string.push_str("\\r"),
                            '\t' => serialized_string.push_str("\\t"),
                            '\\' => serialized_string.push_str("\\"),
                            c if c == quote => {
                                should_quote = true;
                                serialized_string.push_str("\\");
                                serialized_string.push(c);
                            }
                            c if c == opposite_quote => {
                                should_quote = true;
                                serialized_string.push(c);
                            }
                            c => {
                                if c.is_whitespace() {
                                    should_quote = true;
                                }
                                serialized_string.push(c);
                            }
                        }
                    }

                    if should_quote {
                        serialized_string.insert(0, quote);
                        serialized_string.push(quote);
                    }

                    ExpectedValue {
                        expected: Value::String(expected_string),
                        serialized: serialized_string,
                    }
                })
        }

        fn val_strategy() -> impl Strategy<Value = ExpectedValue> {
            prop_oneof![
                int_strategy(),
                double_strategy(),
                bool_strategy(),
                string_strategy(),
            ]
        }

        fn sch_val_behaviour_strategy(val: Value) -> impl Strategy<Value = ValueBehaviour<Value>> {
            prop_oneof![
                Just(ValueBehaviour::Optional),
                Just(ValueBehaviour::Default(val)),
                Just(ValueBehaviour::Required),
            ]
        }

        prop_compose! {
            fn case_strategy()(val in val_strategy())(
                key in key_strategy(),
                val in Just(val.clone()),
                behaviour in sch_val_behaviour_strategy(val.expected.clone()),
                pad_eq_l in any::<bool>(),
                pad_eq_r in any::<bool>(),
            ) -> Case {
                let schema_val = match val.expected.discriminant() {
                    ValueType::Int => SchemaValue::Int(behaviour.map_default(Value::get::<i64>).map_default(Option::unwrap).into()),
                    ValueType::Double => SchemaValue::Double(behaviour.map_default(Value::get::<f64>).map_default(Option::unwrap).into()),
                    ValueType::Bool => SchemaValue::Bool(behaviour.map_default(Value::get::<bool>).map_default(Option::unwrap).into()),
                    ValueType::String => SchemaValue::String(behaviour.map_default(Value::get::<String>).map_default(Option::unwrap).into()),
                    _ => todo!()
                };

                let kv_pair_string = format!(
                    "{}{}={}{}",
                    key.as_ref(),
                    if pad_eq_l { " " } else { "" },
                    if pad_eq_r { " " } else { "" },
                    val.serialized,
                );

                Case {
                    schema_val,
                    key,
                    kv_pair_string,
                    expected: val.expected,
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
                        _ => todo!()
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, SkvEnum)]
    enum TestEnum {
        Variant1,
        Variant2,
        Variant3,
        #[allow(non_camel_case_types)]
        Variant3_X,
    }

    #[test]
    fn enum_value() {
        let mut schema = Schema::default();
        schema.value(key("key.test"), EnumExpectations::from_enum::<TestEnum>());

        let map = ParsedMap::parse(&schema, "key.test=variant1").unwrap();
        assert_eq!(
            map.get_enum::<TestEnum>(&key("key.test")).unwrap(),
            TestEnum::Variant1
        );
    }
}
