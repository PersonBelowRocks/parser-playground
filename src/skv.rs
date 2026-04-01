use std::hash::{BuildHasher, RandomState};

use indexmap::IndexMap;
use winnow::{
    ascii,
    combinator::{cut_err, separated},
    prelude::*,
};

use crate::{
    Key, KeyValuePair, Value,
    error::{ErrorFromParts, MapParseError},
    key_value_pair::skv_pair,
};

pub type Map<H = RandomState> = IndexMap<Key, Value, H>;

/// Parse an SKV map.
#[inline]
#[allow(unused)]
pub fn parse_map(input: impl AsRef<str>) -> Result<Map, MapParseError> {
    parse_map_with_hasher(RandomState::default(), input)
}

/// Parse an SKV map with a custom hasher.
#[inline]
#[allow(unused)]
pub fn parse_map_with_hasher<H: BuildHasher>(
    builder: H,
    input: impl AsRef<str>,
) -> Result<Map<H>, MapParseError> {
    separated(0.., cut_err(skv_pair), ascii::multispace1)
        .parse(input.as_ref().trim())
        .map(|pairs: Vec<KeyValuePair>| {
            let mut map = Map::<H>::with_capacity_and_hasher(pairs.len(), builder);
            for pair in pairs.into_iter() {
                map.insert(pair.key, pair.value);
            }
            map
        })
        .map_err(MapParseError::from_parse_error)
}

#[cfg(test)]
mod tests {
    use crate::{KeyValuePair, Value, kv};

    use super::{Map, parse_map};

    // TODO: more tests

    #[test]
    fn test_parse_map() {
        let input = r#"
            a=1
            b=2.5
            c=true
            d="hello world"
            e=false
        "#;

        let expected = Map::from(
            [
                kv!("a", 1),
                kv!("b", 2.5),
                kv!("c", Value::TRUE),
                kv!("d", "hello world"),
                kv!("e", Value::FALSE),
            ]
            .map(KeyValuePair::into),
        );

        let result = parse_map(input).unwrap();
        assert_eq!(result, expected);
    }
}
