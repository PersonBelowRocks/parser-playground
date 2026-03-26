use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag_no_case,
    combinator::map_res,
    error::{FromExternalError, ParseError},
};

/// Either `true` or `false`, case insensitive.
#[inline(always)]
pub(crate) fn parse_boolean<'a, E>(input: &'a str) -> IResult<&'a str, bool, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::str::ParseBoolError>,
{
    map_res(
        alt((tag_no_case("true"), tag_no_case("false"))),
        // we could just do to_lowercase here but that does an allocation.
        // tbh the allocation will probably be optimized away but i prefer explicitly avoiding allocations where possible
        |s: &str| {
            if s.eq_ignore_ascii_case("true") {
                Ok(true)
            } else if s.eq_ignore_ascii_case("false") {
                Ok(false)
            } else {
                // we can't create the error since it's marked as non_exhaustive, so instead we
                // have the FromStr implementation create the error for us by parsing an invalid bool string
                "".parse::<bool>()
            }
        },
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::parse_boolean;

    const ERROR: nom::Err<()> = nom::Err::Error(());

    #[test]
    fn valid_boolean() {
        // ensure case insensitivity
        assert_eq!(parse_boolean::<()>("true"), Ok(("", true)));
        assert_eq!(parse_boolean::<()>("false"), Ok(("", false)));
        assert_eq!(parse_boolean::<()>("True"), Ok(("", true)));
        assert_eq!(parse_boolean::<()>("False"), Ok(("", false)));
        assert_eq!(parse_boolean::<()>("TRUE"), Ok(("", true)));
        assert_eq!(parse_boolean::<()>("FALSE"), Ok(("", false)));
        // ensure the correct amount is consumed
        assert_eq!(parse_boolean::<()>("true "), Ok((" ", true)));
        assert_eq!(parse_boolean::<()>("truest"), Ok(("st", true)));
        assert_eq!(parse_boolean::<()>("false "), Ok((" ", false)));
        assert_eq!(parse_boolean::<()>("falsest"), Ok(("st", false)));
    }

    #[test]
    fn invalid_boolean() {
        assert_eq!(parse_boolean::<()>(""), Err(ERROR));
        assert_eq!(parse_boolean::<()>(" true"), Err(ERROR));
        assert_eq!(parse_boolean::<()>(" false"), Err(ERROR));
        assert_eq!(parse_boolean::<()>("tru"), Err(ERROR));
        assert_eq!(parse_boolean::<()>("fals"), Err(ERROR));
    }
}
