use nom::{
    IResult, Parser,
    branch::alt,
    character::complete::multispace1,
    combinator::{eof, map, peek},
    error::{FromExternalError, ParseError},
    sequence::terminated,
};

use crate::primitives::{parse_boolean, parse_double, parse_integer, parse_string};

/// A value in an SKV map.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Double(f64),
    Int(i64),
    Bool(bool),
}

impl Value {
    /// A `true` boolean value
    pub const TRUE: Value = Value::Bool(true);
    /// A `false` boolean value
    pub const FALSE: Value = Value::Bool(false);

    /// Create a [`Value::String`] with the argument string.
    ///
    /// Equivalent to `Value::String(s.to_string())`
    ///
    /// # Allocations
    /// Watch out for implementations of [`ToString`] that may allocate.
    #[inline]
    pub fn string<S: ToString>(s: S) -> Self {
        Self::String(s.to_string())
    }

    /// Equivalent to `Value::Double(d.into())`
    #[inline]
    pub fn double<D: Into<f64>>(d: D) -> Self {
        Self::Double(d.into())
    }

    /// Equivalent to `Value::Int(i.into())`
    #[inline]
    pub fn int<I: Into<i64>>(i: I) -> Self {
        Self::Int(i.into())
    }
}

#[allow(unused)]
#[inline(always)]
pub(crate) fn skv_value<'a, E>(input: &'a str) -> IResult<&'a str, Value, E>
where
    E: ParseError<&'a str>
        + FromExternalError<&'a str, std::num::ParseIntError>
        + FromExternalError<&'a str, std::num::TryFromIntError>
        + FromExternalError<&'a str, std::num::ParseFloatError>
        + FromExternalError<&'a str, std::str::ParseBoolError>,
{
    alt((
        // we do this termination logic to disambiguate between a boolean and an unquoted string that starts with "true" or "false".
        // we also can't consume the terminating whitespace since it could be a separator between this value and a key, so we use peek()
        map(
            terminated(parse_boolean, peek(multispace1).or(eof)),
            Value::Bool,
        ),
        // same as with the boolean, we need to disambiguate
        map(
            terminated(parse_integer, peek(multispace1).or(eof)),
            Value::Int,
        ),
        // we can't use nom's default double parser since it freaks out when there's a trailing exponent character (i.e., `e`),
        // so this parser is a modified version of the default one
        map(
            terminated(parse_double, peek(multispace1).or(eof)),
            Value::Double,
        ),
        map(parse_string, Value::String),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::skv_value;
    use crate::Value;

    #[test]
    fn string_boolean_disambiguation() {
        // whitespace acts as a terminator for the boolean
        assert_eq!(skv_value::<()>("true "), Ok((" ", Value::TRUE)));
        assert_eq!(skv_value::<()>("true\n"), Ok(("\n", Value::TRUE)));
        // this string has no whitespace, rather the newline is created during parsing, therefore it's treated as a string
        assert_eq!(
            skv_value::<()>(r#""true\n""#),
            Ok(("", Value::string("true\n")))
        );
        // these start with boolean values, but don't have any whitespace to terminate them
        assert_eq!(
            skv_value::<()>("truestring"),
            Ok(("", Value::string("truestring")))
        );
        assert_eq!(
            skv_value::<()>("falsestring"),
            Ok(("", Value::string("falsestring")))
        );
    }

    #[test]
    fn string_int_disambiguation() {
        assert_eq!(skv_value::<()>("150 test"), Ok((" test", Value::int(150))));
        assert_eq!(
            skv_value::<()>("150test"),
            Ok(("", Value::string("150test")))
        );
        assert_eq!(
            skv_value::<()>("150test next"),
            Ok((" next", Value::string("150test")))
        );
        assert_eq!(
            skv_value::<()>("0xff test"),
            Ok((" test", Value::int(0xff)))
        );
        assert_eq!(
            skv_value::<()>("0xfftest"),
            Ok(("", Value::string("0xfftest")))
        );
    }

    #[test]
    fn string_double_disambiguation() {
        assert_eq!(skv_value::<()>("1.5 "), Ok((" ", Value::double(1.5))));
        assert_eq!(skv_value::<()>("1.5"), Ok(("", Value::double(1.5))));
        assert_eq!(
            skv_value::<()>("1.5test"),
            Ok(("", Value::string("1.5test")))
        );

        // trailing exponents
        assert_eq!(skv_value::<()>("1.5e10"), Ok(("", Value::double(1.5e10))));
        assert_eq!(skv_value::<()>("1.5e"), Ok(("", Value::string("1.5e"))));
    }

    #[test]
    fn valid_value() {
        // normal booleans
        assert_eq!(skv_value::<()>("true"), Ok(("", Value::TRUE)));
        assert_eq!(skv_value::<()>("True"), Ok(("", Value::TRUE)));
        assert_eq!(skv_value::<()>("false"), Ok(("", Value::FALSE)));
        assert_eq!(skv_value::<()>("False"), Ok(("", Value::FALSE)));

        // normal strings
        assert_eq!(
            skv_value::<()>("string!"),
            Ok(("", Value::string("string!")))
        );
        assert_eq!(
            skv_value::<()>("space! "),
            Ok((" ", Value::string("space!")))
        );
        assert_eq!(
            skv_value::<()>(r#""quoted string""#),
            Ok(("", Value::string("quoted string")))
        );
        assert_eq!(
            skv_value::<()>(r#"'single quoted string'"#),
            Ok(("", Value::string("single quoted string")))
        );
        assert_eq!(
            skv_value::<()>(r#""\u{af0f}""#),
            Ok(("", Value::string("\u{af0f}")))
        );

        // integers
        assert_eq!(skv_value::<()>("150"), Ok(("", Value::int(150))));
        assert_eq!(skv_value::<()>("150 "), Ok((" ", Value::int(150))));
        assert_eq!(skv_value::<()>("-66"), Ok(("", Value::int(-66))));
        assert_eq!(
            skv_value::<()>("0b11001100"),
            Ok(("", Value::int(0b11001100)))
        );
    }
}
