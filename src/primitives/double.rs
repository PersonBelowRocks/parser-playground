/// This is a slightly modified version of nom's default float/double parser.
/// The default parser fails when the float/double has a trailing exponent character (`e` or `E`):
/// https://github.com/rust-bakery/nom/issues/1021
///
/// According to the GitHub issue, this is intentional, so we need to implement our own parser for our use case.
use winnow::{
    ascii::{self, Caseless, digit1},
    combinator::{alt, cut_err, eof, opt, peek, terminated},
    prelude::*,
    token::literal,
};

use crate::{expected, label};

/// Modified version of nom's default double parser that doesn't cause a [`nom::Err::Failure`] on trailing exponent characters.
///
/// NaN and infinity can be specified by prefixing them with `\$`: `\$nan`, `\$inf`, `\$NaN`, `\$infinity`, etc. (case insensitive)
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_double(input: &mut &str) -> ModalResult<f64> {
    recognize_float_or_exceptions_allow_trailing_e
        .try_map(str::parse::<f64>)
        .context(label("decimal number"))
        .parse_next(input)
}

#[inline(always)]
fn recognize_float_or_exceptions_allow_trailing_e<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((
        recognize_float_allow_trailing_e,
        literal(Caseless(r#"\$nan"#)).value("nan"),
        literal(Caseless(r#"\$infinity"#)).value("infinity"),
        literal(Caseless(r#"\$inf"#)).value("inf"),
        literal(Caseless(r#"-\$infinity"#)).value("-infinity"),
        literal(Caseless(r#"-\$inf"#)).value("-inf"),
    ))
    .parse_next(input)
}

/// Adapted from https://docs.rs/nom/8.0.0/nom/number/complete/fn.recognize_float.html
#[inline(always)]
fn recognize_float_allow_trailing_e<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    (
        opt(alt((literal('+'), literal('-')))),
        alt((
            (digit1, opt((literal('.'), opt(digit1)))).void(),
            (literal('.'), digit1).void(),
        )),
        opt((
            alt((literal('e'), literal('E'))),
            opt(alt((literal('+'), literal('-')))),
            cut_err(
                terminated(digit1, alt((peek(ascii::multispace1), eof)))
                    .context(expected("an integer exponent")),
            ),
        )),
    )
        .take()
        .parse_next(input)
}
