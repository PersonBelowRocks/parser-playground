use derive_more::{AsRef, Into};
use winnow::{
    ascii::digit1,
    combinator::{cut_err, not, peek, preceded, separated},
    prelude::*,
    token::{literal, take_while},
};

use crate::{
    error::{ErrorFromParts, KeyParseError},
    expected, label,
};

/// A key in an SKV map.
///
/// Keys are made up of key parts, separated by `.`.
/// Key parts are non-empty strings that consist of alphanumeric characters and underscores,
/// except for the first character which is a non-digit.
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Into)]
pub struct Key(String);

impl Key {
    /// Creates a new `Key` from a string slice.
    ///
    /// This function wraps the `FromStr` implementation for `Key`.
    #[inline]
    pub fn new(s: impl AsRef<str>) -> Result<Self, KeyParseError> {
        s.as_ref().parse::<Self>()
    }

    /// Returns an iterator over the parts of the key.
    #[inline]
    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.as_ref().split('.')
    }
}

impl std::str::FromStr for Key {
    type Err = KeyParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        skv_key.parse(s).map_err(KeyParseError::from_parse_error)
    }
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn skv_key(input: &mut &str) -> ModalResult<Key> {
    separated(1.., cut_err(key_part), literal('.'))
        .map(|parts: Vec<&str>| parts.join("."))
        .map(Key)
        .context(label("key"))
        .parse_next(input)
}

#[inline(always)]
fn key_part<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    preceded(
        peek(not(digit1)).context(expected("a non-digit character")),
        take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_')
            .context(expected("a non-empty alphanumeric string")),
    )
    .verify(|s: &str| !s.is_empty())
    .context(label("key part"))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{Key, key_part, skv_key};

    fn key(s: impl Into<String>) -> Key {
        Key(s.into())
    }

    #[test]
    fn valid_key() {
        assert_eq!(skv_key(&mut "test"), Ok(key("test")));
        assert_eq!(skv_key(&mut "test "), Ok(key("test")));
        assert_eq!(skv_key(&mut "test.sep"), Ok(key("test.sep")));
        assert_eq!(skv_key(&mut "test_1.sep2"), Ok(key("test_1.sep2")));
        assert_eq!(
            skv_key(&mut "test_1.test2._test_3"),
            Ok(key("test_1.test2._test_3"))
        );
        assert_eq!(skv_key(&mut "test.sep2"), Ok(key("test.sep2")));
        assert_eq!(skv_key(&mut "test_1.sep"), Ok(key("test_1.sep")));
        assert_eq!(skv_key(&mut "t3st1 .sep2"), Ok(key("t3st1")));
        assert_eq!(skv_key(&mut "t3st1 100"), Ok(key("t3st1")));
        assert_eq!(skv_key(&mut "test1="), Ok(key("test1")));

        assert_eq!(skv_key(&mut "_="), Ok(key("_")));
        assert_eq!(skv_key(&mut "___.___="), Ok(key("___.___")));
        assert_eq!(skv_key(&mut "___="), Ok(key("___")));
        assert_eq!(skv_key(&mut "___"), Ok(key("___")));
    }

    #[test]
    fn invalid_key() {
        assert!(skv_key(&mut "").is_err());
        assert!(skv_key(&mut ".").is_err());
        assert!(skv_key(&mut "..").is_err());
        assert!(skv_key(&mut "1").is_err());
        assert!(skv_key(&mut ".preceding").is_err());
        assert!(skv_key(&mut ".preceding.two").is_err());
        assert!(skv_key(&mut "terminated.").is_err());
        assert!(skv_key(&mut "terminated.two.").is_err());
        assert!(skv_key(&mut "1number").is_err());
        assert!(skv_key(&mut "parts.1number").is_err());
        assert!(skv_key(&mut "parts.1").is_err());
        assert!(skv_key(&mut "parts..").is_err());
    }

    #[test]
    fn valid_key_part() {
        assert_eq!(key_part(&mut "test"), Ok("test"));
        assert_eq!(key_part(&mut "t1"), Ok("t1"));
        assert_eq!(key_part(&mut "test text"), Ok("test"));
        assert_eq!(key_part(&mut "t1 "), Ok("t1"));
        assert_eq!(key_part(&mut "t1_a t1_b"), Ok("t1_a"));
        assert_eq!(key_part(&mut "t1_a.t1_b"), Ok("t1_a"));
    }

    #[test]
    fn invalid_key_part() {
        assert!(key_part(&mut "").is_err());
        assert!(key_part(&mut "1").is_err());
        assert!(key_part(&mut ".").is_err());
        assert!(key_part(&mut " ").is_err());
        assert!(key_part(&mut "1key").is_err());
    }
}
