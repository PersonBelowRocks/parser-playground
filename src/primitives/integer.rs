use winnow::{
    ascii::{Caseless, digit1, hex_digit1, oct_digit1},
    combinator::{alt, preceded},
    prelude::*,
    token::{literal, take_while},
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
pub(crate) fn parse_integer(input: &mut &str) -> ModalResult<i64> {
    alt((
        preceded(literal("-"), unsigned_integer)
            .map(i128::from)
            .map(i128::strict_neg)
            .try_map(i64::try_from),
        unsigned_integer.map(i128::from).try_map(i64::try_from),
    ))
    .parse_next(input)
}

#[inline(always)]
fn unsigned_integer(input: &mut &str) -> ModalResult<u64> {
    alt((
        preceded(literal(Caseless("0x")), int_hex),
        preceded(literal(Caseless("0o")), int_oct),
        preceded(literal(Caseless("0b")), int_bin),
        int_dec,
    ))
    .parse_next(input)
}

/// A hexadecimal integer (e.g. `1A3F`)
#[inline(always)]
fn int_hex(input: &mut &str) -> ModalResult<u64> {
    hex_digit1
        .try_map(|s: &str| u64::from_str_radix(s, 16))
        .parse_next(input)
}

/// An octal integer (e.g. `0755`)
#[inline(always)]
fn int_oct(input: &mut &str) -> ModalResult<u64> {
    oct_digit1
        .try_map(|s: &str| u64::from_str_radix(s, 8))
        .parse_next(input)
}

/// A binary integer (e.g. `1010`)
#[inline(always)]
fn int_bin(input: &mut &str) -> ModalResult<u64> {
    take_while(1.., ['0', '1'])
        .try_map(|s: &str| u64::from_str_radix(s, 2))
        .parse_next(input)
}

/// A decimal integer (e.g. `1234567890`)
#[inline(always)]
fn int_dec(input: &mut &str) -> ModalResult<u64> {
    digit1.try_map(str::parse::<u64>).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::parse_integer;

    #[test]
    fn invalid_integer() {
        assert!(parse_integer(&mut "").is_err());
        assert!(parse_integer(&mut "-").is_err());
        assert!(parse_integer(&mut " ").is_err());
        assert!(parse_integer(&mut "meow").is_err());
        assert!(parse_integer(&mut "").is_err());
    }

    #[test]
    fn valid_hex_integer() {
        // positive / unsigned
        assert_eq!(parse_integer(&mut "0X1"), Ok(0x1));
        assert_eq!(parse_integer(&mut "0x1"), Ok(0x1));
        assert_eq!(parse_integer(&mut "0x01"), Ok(0x01));
        assert_eq!(parse_integer(&mut "0xff"), Ok(0xff));
        assert_eq!(parse_integer(&mut "0X3a"), Ok(0x3a));
        // negative
        assert_eq!(parse_integer(&mut "-0X1"), Ok(-0x1));
        assert_eq!(parse_integer(&mut "-0x1"), Ok(-0x1));
        assert_eq!(parse_integer(&mut "-0x01"), Ok(-0x01));
        assert_eq!(parse_integer(&mut "-0xff"), Ok(-0xff));
        assert_eq!(parse_integer(&mut "-0X3a"), Ok(-0x3a));
        // ensure correct amount is consumed
        assert_eq!(parse_integer(&mut "0xff "), Ok(0xff));
        assert_eq!(parse_integer(&mut "0xffq"), Ok(0xff));
        assert_eq!(parse_integer(&mut "0xffa"), Ok(0xffa));
        assert_eq!(parse_integer(&mut "0xffx"), Ok(0xff));
        assert_eq!(parse_integer(&mut "0xff-"), Ok(0xff));
        assert_eq!(parse_integer(&mut "-0xff-"), Ok(-0xff));
    }

    #[test]
    fn valid_oct_integer() {
        // positive / unsigned
        assert_eq!(parse_integer(&mut "0o1"), Ok(0o1));
        assert_eq!(parse_integer(&mut "0O1"), Ok(0o1));
        assert_eq!(parse_integer(&mut "0o01"), Ok(0o01));
        assert_eq!(parse_integer(&mut "0o77"), Ok(0o77));
        assert_eq!(parse_integer(&mut "0o70"), Ok(0o70));
        // negative
        assert_eq!(parse_integer(&mut "-0o1"), Ok(-0o1));
        assert_eq!(parse_integer(&mut "-0O1"), Ok(-0o1));
        assert_eq!(parse_integer(&mut "-0o01"), Ok(-0o01));
        assert_eq!(parse_integer(&mut "-0o77"), Ok(-0o77));
        assert_eq!(parse_integer(&mut "-0o70"), Ok(-0o70));
        // ensure correct amount is consumed
        assert_eq!(parse_integer(&mut "0o77 "), Ok(0o77));
        assert_eq!(parse_integer(&mut "0o77q"), Ok(0o77));
        assert_eq!(parse_integer(&mut "0o778"), Ok(0o77));
        assert_eq!(parse_integer(&mut "0o77o"), Ok(0o77));
        assert_eq!(parse_integer(&mut "0o77-"), Ok(0o77));
        assert_eq!(parse_integer(&mut "-0o77-"), Ok(-0o77));
    }

    #[test]
    fn valid_bin_integer() {
        // positive / unsigned
        assert_eq!(parse_integer(&mut "0b1"), Ok(0b1));
        assert_eq!(parse_integer(&mut "0B1"), Ok(0b1));
        assert_eq!(parse_integer(&mut "0b01"), Ok(0b01));
        assert_eq!(parse_integer(&mut "0b11111111"), Ok(0b11111111));
        assert_eq!(parse_integer(&mut "0b10"), Ok(0b10));
        // negative
        assert_eq!(parse_integer(&mut "-0b1"), Ok(-0b1));
        assert_eq!(parse_integer(&mut "-0B1"), Ok(-0b1));
        assert_eq!(parse_integer(&mut "-0b01"), Ok(-0b01));
        assert_eq!(parse_integer(&mut "-0b11111111"), Ok(-0b11111111));
        assert_eq!(parse_integer(&mut "-0b10"), Ok(-0b10));
        // ensure correct amount is consumed
        assert_eq!(parse_integer(&mut "0b11 "), Ok(0b11));
        assert_eq!(parse_integer(&mut "0b11q"), Ok(0b11));
        assert_eq!(parse_integer(&mut "0b112"), Ok(0b11));
        assert_eq!(parse_integer(&mut "0b11b"), Ok(0b11));
        assert_eq!(parse_integer(&mut "0b11-"), Ok(0b11));
        assert_eq!(parse_integer(&mut "-0b11-"), Ok(-0b11));
    }

    #[test]
    fn valid_decimal_integer() {
        // positive / unsigned
        assert_eq!(parse_integer(&mut "1"), Ok(1));
        assert_eq!(parse_integer(&mut "01"), Ok(01));
        assert_eq!(parse_integer(&mut "9999"), Ok(9999));
        assert_eq!(parse_integer(&mut "90"), Ok(90));
        // negative
        assert_eq!(parse_integer(&mut "-1"), Ok(-1));
        assert_eq!(parse_integer(&mut "-01"), Ok(-01));
        assert_eq!(parse_integer(&mut "-9999"), Ok(-9999));
        assert_eq!(parse_integer(&mut "-90"), Ok(-90));
        // ensure correct amount is consumed
        assert_eq!(parse_integer(&mut "99 "), Ok(99));
        assert_eq!(parse_integer(&mut "99q"), Ok(99));
        assert_eq!(parse_integer(&mut "99a"), Ok(99));
        assert_eq!(parse_integer(&mut "99-"), Ok(99));
        assert_eq!(parse_integer(&mut "-99-"), Ok(-99));
    }

    #[test]
    fn hex_integer_limits() {
        // maximum
        assert_eq!(parse_integer(&mut "0x7fffffffffffffff"), Ok(i64::MAX));
        assert!(parse_integer(&mut "0x8000000000000000").is_err());
        // minimum
        assert_eq!(parse_integer(&mut "-0x8000000000000000"), Ok(i64::MIN));
        assert!(parse_integer(&mut "-0x8000000000000001").is_err());
    }

    #[test]
    fn oct_integer_limits() {
        // maximum
        assert_eq!(parse_integer(&mut "0o777777777777777777777"), Ok(i64::MAX));
        assert!(parse_integer(&mut "0o1000000000000000000000").is_err());
        // minimum
        assert_eq!(
            parse_integer(&mut "-0o1000000000000000000000"),
            Ok(i64::MIN)
        );
        assert!(parse_integer(&mut "-0o1000000000000000000001").is_err());
    }

    #[test]
    fn bin_integer_limits() {
        let mut max = "0b111111111111111111111111111111111111111111111111111111111111111";
        let mut max_p1 = "0b1000000000000000000000000000000000000000000000000000000000000000";
        let mut min = "-0b1000000000000000000000000000000000000000000000000000000000000000";
        let mut min_m1 = "-0b1000000000000000000000000000000000000000000000000000000000000001";

        // maximum
        assert_eq!(parse_integer(&mut max), Ok(i64::MAX));
        assert!(parse_integer(&mut max_p1).is_err());
        // minimum
        assert_eq!(parse_integer(&mut min), Ok(i64::MIN));
        assert!(parse_integer(&mut min_m1).is_err());
    }

    #[test]
    fn decimal_integer_limits() {
        // maximum
        assert_eq!(parse_integer(&mut "9223372036854775807"), Ok(i64::MAX));
        assert!(parse_integer(&mut "9223372036854775808").is_err());
        // minimum
        assert_eq!(parse_integer(&mut "-9223372036854775808"), Ok(i64::MIN));
        assert!(parse_integer(&mut "-9223372036854775809").is_err());
    }

    #[test]
    fn invalid_hex_integer() {
        assert_eq!(parse_integer(&mut "0x"), Ok(0));
        assert_eq!(parse_integer(&mut "0X"), Ok(0));
        assert_eq!(parse_integer(&mut "0xq"), Ok(0));
        assert_eq!(parse_integer(&mut "-0x"), Ok(0));
        assert_eq!(parse_integer(&mut "-0X"), Ok(0));
        assert_eq!(parse_integer(&mut "-0xq"), Ok(0));
    }

    #[test]
    fn invalid_oct_integer() {
        assert_eq!(parse_integer(&mut "0o"), Ok(0));
        assert_eq!(parse_integer(&mut "0O"), Ok(0));
        assert_eq!(parse_integer(&mut "0oq"), Ok(0));
        assert_eq!(parse_integer(&mut "-0o"), Ok(0));
        assert_eq!(parse_integer(&mut "-0O"), Ok(0));
        assert_eq!(parse_integer(&mut "-0oq"), Ok(0));
    }

    #[test]
    fn invalid_bin_integer() {
        assert_eq!(parse_integer(&mut "0b"), Ok(0));
        assert_eq!(parse_integer(&mut "0B"), Ok(0));
        assert_eq!(parse_integer(&mut "0bq"), Ok(0));
        assert_eq!(parse_integer(&mut "-0b"), Ok(0));
        assert_eq!(parse_integer(&mut "-0B"), Ok(0));
        assert_eq!(parse_integer(&mut "-0bq"), Ok(0));
    }
}
