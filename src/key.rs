use derive_more::{AsRef, Into};
use miette::{Diagnostic, SourceSpan};
use nom::{
    Finish, IResult, Parser, bytes::complete::take_while1, character::complete::{char, digit1}, combinator::{not, peek, verify}, error::ParseError, multi::separated_list1, sequence::preceded
};
use thiserror::Error;

use crate::Span;

/// A key in an SKV map.
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Into)]
pub struct Key(String);

impl Key {
}

#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
#[error("{kind}")]
pub struct KeyParseError {
    #[source]
    pub kind: KeyParseErrorKind,
    #[label("{kind}")]
    pub span: SourceSpan,
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum KeyParseErrorKind {
    #[error("'{0}' is not allowed in keys")]
    IllegalCharacter(char),
    #[error("key cannot start with a numeral")]
    LeadingNumeral,
    #[error("invalid key")]
    Invalid
}

impl std::str::FromStr for Key {
    type Err = KeyParseError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let span = Span::new(s);
        skv_key(span).finish()
    }
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn skv_key<'a, I, E>(input: I) -> IResult<I, Key, E>
where
    E: ParseError<&'a str>,
{
    separated_list1(char('.'), key_part)
        .map(|parts| parts.join("."))
        .map(Key)
        .parse(input)
}

#[inline(always)]
fn key_part<'a, E>(input: Span<'a>) -> IResult<Span<'a>, &'a str, E>
where
    E: ParseError<Span<'a>>,
{
    verify(
        preceded(
            peek(not(digit1::<_, E>)),
            take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_'),
        ),
        |s: Span<'a>| !s.is_empty(),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::{skv_key, key_part, Key};

    const ERROR: nom::Err<()> = nom::Err::Error(());

    fn key(s: impl Into<String>) -> Key {
        Key(s.into())
    }

    #[test]
    fn valid_key() {
        assert_eq!(skv_key::<()>("test"), Ok(("", key("test"))));
        assert_eq!(skv_key::<()>("test "), Ok((" ", key("test"))));
        assert_eq!(skv_key::<()>("test.sep"), Ok(("", key("test.sep"))));
        assert_eq!(skv_key::<()>("test_1.sep2"), Ok(("", key("test_1.sep2"))));
        assert_eq!(skv_key::<()>("test_1.test2._test_3"), Ok(("", key("test_1.test2._test_3"))));
        assert_eq!(skv_key::<()>("test.sep2"), Ok(("", key("test.sep2"))));
        assert_eq!(skv_key::<()>("test_1.sep"), Ok(("", key("test_1.sep"))));
        assert_eq!(skv_key::<()>("t3st1 .sep2"), Ok((" .sep2", key("t3st1"))));
        assert_eq!(skv_key::<()>("t3st1 100"), Ok((" 100", key("t3st1"))));
        assert_eq!(skv_key::<()>("test1="), Ok(("=", key("test1"))));

        assert_eq!(skv_key::<()>("_="), Ok(("=", key("_"))));
        assert_eq!(skv_key::<()>("___.___="), Ok(("=", key("___.___"))));
        assert_eq!(skv_key::<()>("___="), Ok(("=", key("___"))));
        assert_eq!(skv_key::<()>("___"), Ok(("", key("___"))));

        assert_eq!(skv_key::<()>("part.1number"), Ok((".1number", key("part"))));
        assert_eq!(skv_key::<()>("part.1"), Ok((".1", key("part"))));
        assert_eq!(skv_key::<()>("part."), Ok((".", key("part"))));
        assert_eq!(skv_key::<()>("part.."), Ok(("..", key("part"))));
    }

    #[test]
    fn invalid_key() {
        assert_eq!(skv_key::<()>(""), Err(ERROR));
        assert_eq!(skv_key::<()>("."), Err(ERROR));
        assert_eq!(skv_key::<()>(".."), Err(ERROR));
        assert_eq!(skv_key::<()>("1"), Err(ERROR));
        assert_eq!(skv_key::<()>(".preceding"), Err(ERROR));
        assert_eq!(skv_key::<()>("1number"), Err(ERROR));
    }

    #[test]
    fn valid_key_part() {
        assert_eq!(key_part::<()>("test"), Ok(("", "test")));
        assert_eq!(key_part::<()>("t1"), Ok(("", "t1")));
        assert_eq!(key_part::<()>("test text"), Ok((" text", "test")));
        assert_eq!(key_part::<()>("t1 "), Ok((" ", "t1")));
        assert_eq!(key_part::<()>("t1."), Ok((".", "t1")));
        assert_eq!(key_part::<()>("t1. "), Ok((". ", "t1")));
        assert_eq!(key_part::<()>("t1_a t1_b"), Ok((" t1_b", "t1_a")));
        assert_eq!(key_part::<()>("t1_a.t1_b"), Ok((".t1_b", "t1_a")));
    }

    #[test]
    fn invalid_key_part() {
        assert_eq!(key_part::<()>(""), Err(ERROR));
        assert_eq!(key_part::<()>("1"), Err(ERROR));
        assert_eq!(key_part::<()>("."), Err(ERROR));
        assert_eq!(key_part::<()>(" "), Err(ERROR));
        assert_eq!(key_part::<()>("1key"), Err(ERROR));
    }
}