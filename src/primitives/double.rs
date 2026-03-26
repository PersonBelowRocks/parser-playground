use std::num::ParseFloatError;

/// This is a slightly modified version of nom's default float/double parser.
/// The default parser fails when the float/double has a trailing exponent character (`e` or `E`):
/// https://github.com/rust-bakery/nom/issues/1021
///
/// According to the GitHub issue, this is intentional, so we need to implement our own parser for our use case.
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{char, digit1},
    combinator::{map, opt, recognize},
    error::{ErrorKind, FromExternalError, ParseError},
    sequence::pair,
};

/// Modified version of nom's default double parser that doesn't cause a [`nom::Err::Failure`] on trailing exponent characters.
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_double<'a, E>(input: &'a str) -> IResult<&'a str, f64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, ParseFloatError>,
{
    recognize_float_or_exceptions_allow_trailing_e
        .map_res(|s: &str| s.parse::<f64>())
        .parse(input)
}

#[inline(always)]
fn recognize_float_or_exceptions_allow_trailing_e<'a, E>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str>,
{
    alt((
        recognize_float_allow_trailing_e,
        |i: &'a str| {
            tag_no_case::<_, _, E>("nan")(i)
                .map_err(|_| nom::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
        },
        |i: &'a str| {
            tag_no_case::<_, _, E>("infinity")(i)
                .map_err(|_| nom::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
        },
        |i: &'a str| {
            tag_no_case::<_, _, E>("inf")(i)
                .map_err(|_| nom::Err::Error(E::from_error_kind(i, ErrorKind::Float)))
        },
    ))
    .parse(input)
}

/// Adapted from https://docs.rs/nom/8.0.0/nom/number/complete/fn.recognize_float.html
#[inline(always)]
fn recognize_float_allow_trailing_e<'a, E>(input: &'a str) -> IResult<&'a str, &'a str, E>
where
    E: ParseError<&'a str>,
{
    recognize((
        opt(alt((char('+'), char('-')))),
        alt((
            map((digit1, opt(pair(char('.'), opt(digit1)))), |_| ()),
            map((char('.'), digit1), |_| ()),
        )),
        opt((
            alt((char('e'), char('E'))),
            opt(alt((char('+'), char('-')))),
            // this line had a cut() on it, which caused a failure on a trailing E
            digit1,
        )),
    ))
    .parse(input)
}
