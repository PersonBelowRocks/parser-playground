use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
};

use indexmap::IndexMap;
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

pub(crate) type InnerMap<H = RandomState> = IndexMap<Key, Value, H>;

#[derive(Clone, Debug, derive_more::Into)]
pub struct ParsedMap<H = RandomState>(IndexMap<Key, Value, H>);

impl ParsedMap {
    #[inline]
    pub fn parse(schema: &Schema, input: impl AsRef<str>) -> Result<Self, MapParseError> {
        Self::parse_with_hasher(RandomState::default(), schema, input)
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
    use crate::util::testing::key;
    use crate::{Schema, ValueBehaviour};

    use super::ParsedMap;

    // TODO: more tests, especially around errors
    #[test]
    fn required_value() {
        let mut schema = Schema::default();
        schema.value(key("key.test"), ValueBehaviour::<i64>::Required);

        let map = ParsedMap::parse(&schema, "key.test=42").unwrap();
        assert_eq!(map.get_int(&key("key.test")).unwrap(), 42);

        // missing key
        assert!(ParsedMap::parse(&schema, "not.key.test=42").is_err());
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
