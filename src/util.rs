use std::ops::{Bound, RangeBounds};

use winnow::{
    ascii::multispace1,
    combinator::{alt, eof, peek, terminated},
    error::{ParserError, StrContext, StrContextValue},
    prelude::*, stream::Range,
};

#[inline(always)]
pub(crate) fn token<'a, O, E, P>(parser: P) -> impl Parser<&'a str, O, E>
where
    P: Parser<&'a str, O, E>,
    E: ParserError<&'a str>,
{
    terminated(parser, alt((peek(multispace1), eof)))
}

pub(crate) const fn expected(description: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(description))
}

pub(crate) const fn label(description: &'static str) -> StrContext {
    StrContext::Label(description)
}

pub(crate) trait Sealed {}

/// Utilities for use in testing.
#[cfg(test)]
pub(crate) mod testing {
    use crate::Key;

    pub(crate) fn key(s: impl AsRef<str>) -> Key {
        Key::new(s.as_ref()).unwrap()
    }

    #[macro_export]
    macro_rules! kv {
        ($key:expr, $val:expr) => {
            crate::KeyValuePair::new(crate::util::testing::key($key), $val)
        };
    }
}
