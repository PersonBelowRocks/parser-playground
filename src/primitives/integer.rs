use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{bin_digit1, digit1, hex_digit1, oct_digit1},
    combinator::map_res,
    error::{FromExternalError, ParseError},
    sequence::preceded,
};

/// An integer in decimal, hexadecimal, octal, or binary format.
///
/// Negative integers have a leading `-`, before the radix prefix.
///
/// Prefixed with the usual radix prefixes:
/// - `0x` for hexadecimal (e.g. `0x1A3F`)
/// - `0o` for octal (e.g. `0o755`)
/// - `0b` for binary (e.g. `0b1010`)
/// - No prefix for decimal (e.g. `1234567890`)
#[inline(always)]
pub(crate) fn parse_integer<'a, E>(input: &'a str) -> IResult<&'a str, i64, E>
where
    E: ParseError<&'a str>
        + FromExternalError<&'a str, std::num::ParseIntError>
        + FromExternalError<&'a str, std::num::TryFromIntError>,
{
    alt((
        preceded(tag("-"), unsigned_integer)
            .map(i128::from)
            .map(i128::strict_neg)
            .map_res(i64::try_from),
        (unsigned_integer).map(i128::from).map_res(i64::try_from),
    ))
    .parse(input)
}

#[inline(always)]
fn unsigned_integer<'a, E>(input: &'a str) -> IResult<&'a str, u64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((
        preceded(tag_no_case("0x"), int_hex),
        preceded(tag_no_case("0o"), int_oct),
        preceded(tag_no_case("0b"), int_bin),
        int_dec,
    ))
    .parse(input)
}

/// A hexadecimal integer (e.g. `1A3F`)
#[inline(always)]
fn int_hex<'a, E>(input: &'a str) -> IResult<&'a str, u64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    map_res(hex_digit1, |s: &str| u64::from_str_radix(s, 16)).parse(input)
}

/// An octal integer (e.g. `0755`)
#[inline(always)]
fn int_oct<'a, E>(input: &'a str) -> IResult<&'a str, u64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    map_res(oct_digit1, |s: &str| u64::from_str_radix(s, 8)).parse(input)
}

/// A binary integer (e.g. `1010`)
#[inline(always)]
fn int_bin<'a, E>(input: &'a str) -> IResult<&'a str, u64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    map_res(bin_digit1, |s: &str| u64::from_str_radix(s, 2)).parse(input)
}

/// A decimal integer (e.g. `1234567890`)
#[inline(always)]
fn int_dec<'a, E>(input: &'a str) -> IResult<&'a str, u64, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    map_res(digit1, |s: &str| u64::from_str_radix(s, 10)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::parse_integer;

    const ERROR: nom::Err<()> = nom::Err::Error(());

    #[test]
    fn invalid_integer() {
        assert_eq!(parse_integer::<()>(""), Err(ERROR));
        assert_eq!(parse_integer::<()>("-"), Err(ERROR));
        assert_eq!(parse_integer::<()>(" "), Err(ERROR));
        assert_eq!(parse_integer::<()>("meow"), Err(ERROR));
        assert_eq!(parse_integer::<()>(""), Err(ERROR));
    }

    #[test]
    fn valid_hex_integer() {
        // positive / unsigned
        assert_eq!(parse_integer::<()>("0X1"), Ok(("", 0x1)));
        assert_eq!(parse_integer::<()>("0x1"), Ok(("", 0x1)));
        assert_eq!(parse_integer::<()>("0x01"), Ok(("", 0x01)));
        assert_eq!(parse_integer::<()>("0xff"), Ok(("", 0xff)));
        assert_eq!(parse_integer::<()>("0X3a"), Ok(("", 0x3a)));
        // negative
        assert_eq!(parse_integer::<()>("-0X1"), Ok(("", -0x1)));
        assert_eq!(parse_integer::<()>("-0x1"), Ok(("", -0x1)));
        assert_eq!(parse_integer::<()>("-0x01"), Ok(("", -0x01)));
        assert_eq!(parse_integer::<()>("-0xff"), Ok(("", -0xff)));
        assert_eq!(parse_integer::<()>("-0X3a"), Ok(("", -0x3a)));
        // ensure correct amount is consumed
        assert_eq!(parse_integer::<()>("0xff "), Ok((" ", 0xff)));
        assert_eq!(parse_integer::<()>("0xffq"), Ok(("q", 0xff)));
        assert_eq!(parse_integer::<()>("0xffa"), Ok(("", 0xffa)));
        assert_eq!(parse_integer::<()>("0xffx"), Ok(("x", 0xff)));
        assert_eq!(parse_integer::<()>("0xff-"), Ok(("-", 0xff)));
        assert_eq!(parse_integer::<()>("-0xff-"), Ok(("-", -0xff)));
    }

    #[test]
    fn valid_oct_integer() {
        // positive / unsigned
        assert_eq!(parse_integer::<()>("0o1"), Ok(("", 0o1)));
        assert_eq!(parse_integer::<()>("0O1"), Ok(("", 0o1)));
        assert_eq!(parse_integer::<()>("0o01"), Ok(("", 0o01)));
        assert_eq!(parse_integer::<()>("0o77"), Ok(("", 0o77)));
        assert_eq!(parse_integer::<()>("0o70"), Ok(("", 0o70)));
        // negative
        assert_eq!(parse_integer::<()>("-0o1"), Ok(("", -0o1)));
        assert_eq!(parse_integer::<()>("-0O1"), Ok(("", -0o1)));
        assert_eq!(parse_integer::<()>("-0o01"), Ok(("", -0o01)));
        assert_eq!(parse_integer::<()>("-0o77"), Ok(("", -0o77)));
        assert_eq!(parse_integer::<()>("-0o70"), Ok(("", -0o70)));
        // ensure correct amount is consumed
        assert_eq!(parse_integer::<()>("0o77 "), Ok((" ", 0o77)));
        assert_eq!(parse_integer::<()>("0o77q"), Ok(("q", 0o77)));
        assert_eq!(parse_integer::<()>("0o778"), Ok(("8", 0o77)));
        assert_eq!(parse_integer::<()>("0o77o"), Ok(("o", 0o77)));
        assert_eq!(parse_integer::<()>("0o77-"), Ok(("-", 0o77)));
        assert_eq!(parse_integer::<()>("-0o77-"), Ok(("-", -0o77)));
    }

    #[test]
    fn valid_bin_integer() {
        // positive / unsigned
        assert_eq!(parse_integer::<()>("0b1"), Ok(("", 0b1)));
        assert_eq!(parse_integer::<()>("0B1"), Ok(("", 0b1)));
        assert_eq!(parse_integer::<()>("0b01"), Ok(("", 0b01)));
        assert_eq!(parse_integer::<()>("0b11111111"), Ok(("", 0b11111111)));
        assert_eq!(parse_integer::<()>("0b10"), Ok(("", 0b10)));
        // negative
        assert_eq!(parse_integer::<()>("-0b1"), Ok(("", -0b1)));
        assert_eq!(parse_integer::<()>("-0B1"), Ok(("", -0b1)));
        assert_eq!(parse_integer::<()>("-0b01"), Ok(("", -0b01)));
        assert_eq!(parse_integer::<()>("-0b11111111"), Ok(("", -0b11111111)));
        assert_eq!(parse_integer::<()>("-0b10"), Ok(("", -0b10)));
        // ensure correct amount is consumed
        assert_eq!(parse_integer::<()>("0b11 "), Ok((" ", 0b11)));
        assert_eq!(parse_integer::<()>("0b11q"), Ok(("q", 0b11)));
        assert_eq!(parse_integer::<()>("0b112"), Ok(("2", 0b11)));
        assert_eq!(parse_integer::<()>("0b11b"), Ok(("b", 0b11)));
        assert_eq!(parse_integer::<()>("0b11-"), Ok(("-", 0b11)));
        assert_eq!(parse_integer::<()>("-0b11-"), Ok(("-", -0b11)));
    }

    #[test]
    fn valid_decimal_integer() {
        // positive / unsigned
        assert_eq!(parse_integer::<()>("1"), Ok(("", 1)));
        assert_eq!(parse_integer::<()>("01"), Ok(("", 01)));
        assert_eq!(parse_integer::<()>("9999"), Ok(("", 9999)));
        assert_eq!(parse_integer::<()>("90"), Ok(("", 90)));
        // negative
        assert_eq!(parse_integer::<()>("-1"), Ok(("", -1)));
        assert_eq!(parse_integer::<()>("-01"), Ok(("", -01)));
        assert_eq!(parse_integer::<()>("-9999"), Ok(("", -9999)));
        assert_eq!(parse_integer::<()>("-90"), Ok(("", -90)));
        // ensure correct amount is consumed
        assert_eq!(parse_integer::<()>("99 "), Ok((" ", 99)));
        assert_eq!(parse_integer::<()>("99q"), Ok(("q", 99)));
        assert_eq!(parse_integer::<()>("99a"), Ok(("a", 99)));
        assert_eq!(parse_integer::<()>("99-"), Ok(("-", 99)));
        assert_eq!(parse_integer::<()>("-99-"), Ok(("-", -99)));
    }

    #[test]
    fn hex_integer_limits() {
        // maximum
        assert_eq!(
            parse_integer::<()>("0x7fffffffffffffff"),
            Ok(("", i64::MAX))
        );
        assert_eq!(parse_integer::<()>("0x8000000000000000"), Err(ERROR));
        // minimum
        assert_eq!(
            parse_integer::<()>("-0x8000000000000000"),
            Ok(("", i64::MIN))
        );
        assert_eq!(parse_integer::<()>("-0x8000000000000001"), Err(ERROR));
    }

    #[test]
    fn oct_integer_limits() {
        // maximum
        assert_eq!(
            parse_integer::<()>("0o777777777777777777777"),
            Ok(("", i64::MAX))
        );
        assert_eq!(parse_integer::<()>("0o1000000000000000000000"), Err(ERROR));
        // minimum
        assert_eq!(
            parse_integer::<()>("-0o1000000000000000000000"),
            Ok(("", i64::MIN))
        );
        assert_eq!(parse_integer::<()>("-0o1000000000000000000001"), Err(ERROR));
    }

    #[test]
    fn bin_integer_limits() {
        let max = "0b111111111111111111111111111111111111111111111111111111111111111";
        let max_p1 = "0b1000000000000000000000000000000000000000000000000000000000000000";
        let min = "-0b1000000000000000000000000000000000000000000000000000000000000000";
        let min_m1 = "-0b1000000000000000000000000000000000000000000000000000000000000001";

        // maximum
        assert_eq!(parse_integer::<()>(max), Ok(("", i64::MAX)));
        assert_eq!(parse_integer::<()>(max_p1), Err(ERROR));
        // minimum
        assert_eq!(parse_integer::<()>(min), Ok(("", i64::MIN)));
        assert_eq!(parse_integer::<()>(min_m1), Err(ERROR));
    }

    #[test]
    fn decimal_integer_limits() {
        // maximum
        assert_eq!(
            parse_integer::<()>("9223372036854775807"),
            Ok(("", i64::MAX))
        );
        assert_eq!(parse_integer::<()>("9223372036854775808"), Err(ERROR));
        // minimum
        assert_eq!(
            parse_integer::<()>("-9223372036854775808"),
            Ok(("", i64::MIN))
        );
        assert_eq!(parse_integer::<()>("-9223372036854775809"), Err(ERROR));
    }

    #[test]
    fn invalid_hex_integer() {
        assert_eq!(parse_integer::<()>("0x"), Ok(("x", 0)));
        assert_eq!(parse_integer::<()>("0X"), Ok(("X", 0)));
        assert_eq!(parse_integer::<()>("0xq"), Ok(("xq", 0)));
        assert_eq!(parse_integer::<()>("-0x"), Ok(("x", 0)));
        assert_eq!(parse_integer::<()>("-0X"), Ok(("X", 0)));
        assert_eq!(parse_integer::<()>("-0xq"), Ok(("xq", 0)));
    }

    #[test]
    fn invalid_oct_integer() {
        assert_eq!(parse_integer::<()>("0o"), Ok(("o", 0)));
        assert_eq!(parse_integer::<()>("0O"), Ok(("O", 0)));
        assert_eq!(parse_integer::<()>("0oq"), Ok(("oq", 0)));
        assert_eq!(parse_integer::<()>("-0o"), Ok(("o", 0)));
        assert_eq!(parse_integer::<()>("-0O"), Ok(("O", 0)));
        assert_eq!(parse_integer::<()>("-0oq"), Ok(("oq", 0)));
    }

    #[test]
    fn invalid_bin_integer() {
        assert_eq!(parse_integer::<()>("0b"), Ok(("b", 0)));
        assert_eq!(parse_integer::<()>("0B"), Ok(("B", 0)));
        assert_eq!(parse_integer::<()>("0bq"), Ok(("bq", 0)));
        assert_eq!(parse_integer::<()>("-0b"), Ok(("b", 0)));
        assert_eq!(parse_integer::<()>("-0B"), Ok(("B", 0)));
        assert_eq!(parse_integer::<()>("-0bq"), Ok(("bq", 0)));
    }
}
