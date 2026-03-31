use winnow::{ascii::Caseless, combinator::alt, prelude::*, token::literal};

/// Either `true` or `false`, case insensitive.
#[inline(always)]
pub(crate) fn parse_boolean(input: &mut &str) -> ModalResult<bool> {
    alt((literal(Caseless("true")), literal(Caseless("false"))))
        .try_map(
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
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::parse_boolean;

    #[test]
    fn valid_boolean() {
        // ensure case insensitivity
        assert_eq!(parse_boolean(&mut "true"), Ok(true));
        assert_eq!(parse_boolean(&mut "false"), Ok(false));
        assert_eq!(parse_boolean(&mut "True"), Ok(true));
        assert_eq!(parse_boolean(&mut "False"), Ok(false));
        assert_eq!(parse_boolean(&mut "TRUE"), Ok(true));
        assert_eq!(parse_boolean(&mut "FALSE"), Ok(false));
        // ensure the correct amount is consumed
        assert_eq!(parse_boolean(&mut "true "), Ok(true));
        assert_eq!(parse_boolean(&mut "truest"), Ok(true));
        assert_eq!(parse_boolean(&mut "false "), Ok(false));
        assert_eq!(parse_boolean(&mut "falsest"), Ok(false));
    }

    #[test]
    fn invalid_boolean() {
        assert!(parse_boolean(&mut "").is_err());
        assert!(parse_boolean(&mut " true").is_err());
        assert!(parse_boolean(&mut " false").is_err());
        assert!(parse_boolean(&mut "tru").is_err());
        assert!(parse_boolean(&mut "fals").is_err());
    }
}
