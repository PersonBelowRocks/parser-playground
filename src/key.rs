use std::cmp::min;

use derive_more::{AsRef, Into};
use miette::{Diagnostic, SourceOffset, SourceSpan};
use thiserror::Error;
use winnow::{
    ascii::digit1,
    combinator::{cut_err, not, peek, preceded, separated},
    error::{StrContext, StrContextValue},
    prelude::*,
    token::{literal, take_while},
};

/// A key in an SKV map.
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Into)]
pub struct Key(String);

impl Key {}

impl std::str::FromStr for Key {
    type Err = KeyParseError;

    #[inline]
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        skv_key.parse(&mut s).map_err(|error| {
            let span = error.char_span();
            let start = SourceOffset::from(span.start);
            let length = min(1, span.end.saturating_sub(span.start));

            dbg!(error.clone());
            let mut input = error.input().to_string();
            input.push(' ');

            let message = match error.into_inner().to_string().as_str() {
                "" => "error".to_string(),
                msg @ _ => msg.to_string(),
            };

            KeyParseError {
                message,
                input,
                span: SourceSpan::new(start, length),
            }
        })
    }
}

#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
#[error("error parsing key")]
#[diagnostic(code(error::parse::key), help("see docs for key syntax"))]
pub struct KeyParseError {
    pub message: String,
    #[source_code]
    pub input: String,
    #[label("{message}")]
    pub span: SourceSpan,
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn skv_key(input: &mut &str) -> ModalResult<Key> {
    separated(1.., cut_err(key_part), literal('.'))
        .map(|parts: Vec<&str>| parts.join("."))
        .map(Key)
        .parse_next(input)
}

#[inline(always)]
fn key_part<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    preceded(
        peek(not(digit1)).context(StrContext::Expected(StrContextValue::Description(
            "a non-digit character",
        ))),
        take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_').context(
            StrContext::Expected(StrContextValue::Description(
                "a non-empty alphanumeric string",
            )),
        ),
    )
    .verify(|s: &str| !s.is_empty())
    .context(StrContext::Label("key part"))
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

        assert_eq!(skv_key(&mut "part.1number"), Ok(key("part")));
        assert_eq!(skv_key(&mut "part.1"), Ok(key("part")));
        assert_eq!(skv_key(&mut "part."), Ok(key("part")));
        assert_eq!(skv_key(&mut "part.."), Ok(key("part")));
    }

    #[test]
    fn invalid_key() {
        assert!(skv_key(&mut "").is_err());
        assert!(skv_key(&mut ".").is_err());
        assert!(skv_key(&mut "..").is_err());
        assert!(skv_key(&mut "1").is_err());
        assert!(skv_key(&mut ".preceding").is_err());
        assert!(skv_key(&mut "1number").is_err());
    }

    #[test]
    fn valid_key_part() {
        assert_eq!(key_part(&mut "test"), Ok("test"));
        assert_eq!(key_part(&mut "t1"), Ok("t1"));
        assert_eq!(key_part(&mut "test text"), Ok("test"));
        assert_eq!(key_part(&mut "t1 "), Ok("t1"));
        assert_eq!(key_part(&mut "t1."), Ok("t1"));
        assert_eq!(key_part(&mut "t1. "), Ok("t1"));
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
