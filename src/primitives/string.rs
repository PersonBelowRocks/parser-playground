/// Opinionated string parsing code for both quoted and unquoted strings, with support for escape characters and unicode sequences.
/// Code is largely taken from the `nom` string example, with modifications to allow unquoted strings and a handful of other tweaks:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
///
/// # Notes on unicode sequences
/// Unicode sequences look like this: `\u{XXXX}`, where the `X` characters are hex digits. 1 to 6 hex digits are allowed between the braces.
/// All unicode characters are allowed and are escaped (converted into the actual characters in the output),
/// except for the null character (`\u{00}`), which is disallowed and will cause an error if included in the input string.
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, take_while, take_while_m_n},
    character::complete::char,
    combinator::{map, map_opt, map_res, value, verify},
    error::{FromExternalError, ParseError},
    multi::fold,
    sequence::{delimited, preceded},
};

/// Parse a unicode sequence, of the form u{XXXX}, where XXXX is 1 to 6
/// hexadecimal numerals. We will combine this later with parse_escaped_char
/// to parse sequences like \u{00AC}.
///
/// Fails if the character is a null byte (`\u{00}`), we do not want these in our strings!
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn parse_unicode<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // `take_while_m_n` parses between `m` and `n` bytes (inclusive) that match
    // a predicate. `parse_hex` here parses between 1 and 6 hexadecimal numerals.
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());

    // `preceded` takes a prefix parser, and if it succeeds, returns the result
    // of the body parser. In this case, it parses u{XXXX}.
    let parse_delimited_hex = preceded(
        char('u'),
        // `delimited` is like `preceded`, but it parses both a prefix and a suffix.
        // It returns the result of the middle parser. In this case, it parses
        // {XXXX}, where XXXX is 1 to 6 hex numerals, and returns XXXX
        delimited(char('{'), parse_hex, char('}')),
    );

    // `map_res` takes the result of a parser and applies a function that returns
    // a Result. In this case we take the hex bytes from parse_hex and attempt to
    // convert them to a u32.
    let parse_u32 = map_res(parse_delimited_hex, move |hex| u32::from_str_radix(hex, 16));

    // map_opt is like map_res, but it takes an Option instead of a Result. If
    // the function returns None, map_opt returns an error. In this case, because
    // not all u32 values are valid unicode code points, we have to fallibly
    // convert to char with from_u32.
    verify(map_opt(parse_u32, std::char::from_u32), |&c| c != '\0').parse(input)
}

/// Parse an escaped character: \n, \t, \r, \u{00AC}, etc.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn parse_escaped_char<'a, E>(input: &'a str) -> IResult<&'a str, char, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    preceded(
        char('\\'),
        // `alt` tries each parser in sequence, returning the result of
        // the first successful match
        alt((
            parse_unicode,
            // The `value` parser returns a fixed value (the first argument) if its
            // parser (the second argument) succeeds. In these cases, it looks for
            // the marker characters (n, r, t, etc) and returns the matching
            // character (\n, \r, \t, etc).
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
            value('\u{08}', char('b')),
            value('\u{0C}', char('f')),
            value('\\', char('\\')),
            value('/', char('/')),
            value('"', char('"')),
            value('\'', char('\'')),
        )),
    )
    .parse(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuoteType {
    Single,
    Double,
    Unquoted,
}

/// Parse a non-empty block of text that doesn't include \ or "
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn parse_literal<'a, E: ParseError<&'a str>>(
    input: &'a str,
    quote_type: QuoteType,
) -> IResult<&'a str, &'a str, E> {
    // `is_not` parses a string of 0 or more characters that aren't one of the
    // given characters.
    let disallowed_characters = match quote_type {
        QuoteType::Single => ['\'', '\\', '\0'], // r#"'\"# + '\0',
        QuoteType::Double => ['"', '\\', '\0'],  // r#""\"# + '\0',
        QuoteType::Unquoted => {
            // If we're parsing an unquoted string, we also want to disallow whitespace.
            // `not` is a combinator that succeeds if its parser fails, and fails if
            // its parser succeeds. In this case, we want to ensure that the output of
            // not_quote_or_slash doesn't contain any whitespace, so we use not to check
            // that the input doesn't contain any whitespace.
            let parser = take_while(|c: char| {
                !c.is_whitespace() && c != '"' && c != '\'' && c != '\\' && c != '\0'
            });

            return verify(parser, |s: &str| !s.is_empty()).parse(input);
        }
    };

    let not_quote_or_slash = is_not(&disallowed_characters[..]);

    // `verify` runs a parser, then runs a verification function on the output of
    // the parser. The verification function accepts out output only if it
    // returns true. In this case, we want to ensure that the output of is_not
    // is non-empty.
    verify(not_quote_or_slash, |s: &str| !s.is_empty()).parse(input)
}

/// A string fragment contains a fragment of a string being parsed:
/// either a non-empty Literal (a series of non-escaped characters)
/// or a single parsed escaped character.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    Literal(&'a str),
    EscapedChar(char),
}

/// Combine parse_literal and parse_escaped_char into a StringFragment.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn parse_fragment<'a, E>(
    input: &'a str,
    quote_type: QuoteType,
) -> IResult<&'a str, StringFragment<'a>, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((
        // The `map` combinator runs a parser, then applies a function to the output
        // of that parser.
        map(|i| parse_literal(i, quote_type), StringFragment::Literal),
        map(parse_escaped_char, StringFragment::EscapedChar),
    ))
    .parse(input)
}

/// Parser for an unquoted string.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn string_builder<'a, E>(quote_type: QuoteType) -> impl Parser<&'a str, Output = String, Error = E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // fold is the equivalent of iterator::fold. It runs a parser in a loop,
    // and for each output value, calls a folding function on each output value.
    fold(
        0..,
        // Our parser function – parses a single string fragment
        move |i| parse_fragment(i, quote_type),
        // Our init value, an empty string
        String::new,
        // Our folding function. For each fragment, append the fragment to the
        // string.
        |mut string, fragment| {
            match fragment {
                StringFragment::Literal(s) => string.push_str(s),
                StringFragment::EscapedChar(c) => string.push(c),
            }
            string
        },
    )
}

/// Parse a quoted string.
///
/// # Quotation rules
/// - Quotes are either `'` or `"`.
/// - Start quote and end quote must be the same character (i.e., if string starts with `'`, it must end with `'`).
/// - The first character of the input determines which quote should indicate the end of the string.
/// - Quotes can be escaped by using a backslash (`\'` or `\"`), and won't be treated as the end of the string.
/// - A string can contain the other type of quote without escaping (e.g. `"hello 'world'"` is valid and produces `hello 'world'`).
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_quoted_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    // Finally, parse the string. Note that, if `build_string` could accept a raw
    // " character, the closing delimiter " would never match. When using
    // `delimited` with a looping parser (like fold), be sure that the
    // loop won't accidentally match your closing delimiter!
    if input.starts_with('\'') {
        delimited(char('\''), string_builder(QuoteType::Single), char('\'')).parse(input)
    } else {
        delimited(char('"'), string_builder(QuoteType::Double), char('"')).parse(input)
    }
}

/// Parse an unquoted string.
///
/// Will produce an error if the string is empty, contains only whitespace, or contains a quote character (either `'` or `"`).
/// Escaped whitespace characters (e.g. `\t`, `\n`) are allowed will be escaped and included in the output string.
/// Whitespace is otherwise not allowed and will be treated as the end of the string.
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_unquoted_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    verify(string_builder(QuoteType::Unquoted), |s: &str| !s.is_empty()).parse(input)
}

/// Parse a string with or without quotes.
///
/// Determines whether to parse the string as a quoted or an unquoted string based on the starting character of the input:
/// if it starts with a `'` or `"`, it's parsed as a quoted string, otherwise it's parsed as an unquoted string.
///
/// Different rules apply depending on whether the string is parsed as quoted or unquoted.
/// See [parse_quoted_string] and [parse_unquoted_string] for details on differences.
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_string<'a, E>(input: &'a str) -> IResult<&'a str, String, E>
where
    E: ParseError<&'a str> + FromExternalError<&'a str, std::num::ParseIntError>,
{
    alt((parse_quoted_string, parse_unquoted_string)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::{parse_quoted_string, parse_string, parse_unquoted_string};

    macro_rules! assert_eq_quoted {
        ($input:expr, $output:expr, $remaining:expr) => {
            assert_eq!(
                parse_quoted_string::<()>($input),
                Ok(($remaining, str::to_string($output)))
            );
        };
    }

    macro_rules! assert_eq_unquoted {
        ($input:expr, $output:expr, $remaining:expr) => {
            assert_eq!(
                parse_unquoted_string::<()>($input),
                Ok(($remaining, str::to_string($output)))
            );
        };
    }

    #[test]
    fn string() {
        assert_eq!(
            parse_string::<()>(r#""hello world""#),
            Ok(("", "hello world".to_string()))
        );
        assert_eq!(
            parse_string::<()>("'hello world'"),
            Ok(("", "hello world".to_string()))
        );
        assert_eq!(
            parse_string::<()>("hello world"),
            Ok((" world", "hello".to_string()))
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse_string::<()>(""), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>("   "), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>("''"), Ok(("", "".to_string())));
        assert_eq!(parse_string::<()>(r#""""#), Ok(("", "".to_string())));
    }

    #[test]
    fn invalid_string() {
        assert_eq!(parse_string::<()>("'"), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>(r#"""#), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>("'hello world"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_string::<()>(r#""hello world"#),
            Err(nom::Err::Error(()))
        );

        // this syntax is reserved for the double parser
        assert_eq!(parse_string::<()>(r#"\$nan"#), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>(r#"\$inf"#), Err(nom::Err::Error(())));
        assert_eq!(
            parse_string::<()>(r#"\$infinity"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_string::<()>(r#"\$blahblah"#),
            Err(nom::Err::Error(()))
        );
    }

    #[test]
    fn escape_characters() {
        // we don't care if broken syntax exists outside of the string
        assert_eq!(
            parse_string::<()>(r#"hello \ø world"#),
            Ok((r#" \ø world"#, "hello".to_string()))
        );

        // unrecognized escape sequences cause an error
        assert_eq!(parse_string::<()>(r#"\ø"#), Err(nom::Err::Error(())));
        // broken unicode escape syntax causes an error
        assert_eq!(parse_string::<()>(r#"\u"#), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>(r#"\u1234"#), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>(r#"\u{1234"#), Err(nom::Err::Error(())));
        assert_eq!(parse_string::<()>(r#"\umeow"#), Err(nom::Err::Error(())));
        // null character is not allowed even when syntax is correct
        assert_eq!(parse_string::<()>(r#"\u{00}"#), Err(nom::Err::Error(())));

        assert_eq_unquoted!(r#"\u{03A3}"#, "Σ", "");
        assert_eq_unquoted!(r#"\u{6C34}"#, "水", "");
    }

    #[test]
    fn empty_quoted_string() {
        assert_eq_quoted!(r#""""#, "", "");
        assert_eq_quoted!("''", "", "");
        assert_eq!(parse_quoted_string::<()>(r#""""#), Ok(("", "".to_string())));
        assert_eq!(parse_quoted_string::<()>("''"), Ok(("", "".to_string())));
    }

    #[test]
    fn empty_unquoted_string() {
        // unquoted strings can't be empty or whitespace-only
        assert_eq!(parse_unquoted_string::<()>(""), Err(nom::Err::Error(())));
        assert_eq!(parse_unquoted_string::<()>("   "), Err(nom::Err::Error(())));
    }

    #[test]
    fn invalid_quoted_string() {
        // quoted strings require quotes
        assert_eq!(parse_quoted_string::<()>(""), Err(nom::Err::Error(())));
        assert_eq!(parse_quoted_string::<()>("   "), Err(nom::Err::Error(())));
        assert_eq!(parse_quoted_string::<()>("hello"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_quoted_string::<()>("hello world"),
            Err(nom::Err::Error(()))
        );

        // missing quotes cause an error
        // (double quotes)
        assert_eq!(parse_quoted_string::<()>(r#"""#), Err(nom::Err::Error(())));
        assert_eq!(
            parse_quoted_string::<()>(r#""hello world"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_quoted_string::<()>(r#"hello world""#),
            Err(nom::Err::Error(()))
        );
        // (single quotes)
        assert_eq!(parse_quoted_string::<()>("'"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_quoted_string::<()>("'hello world"),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_quoted_string::<()>("hello world'"),
            Err(nom::Err::Error(()))
        );

        // mismatched quotes cause an error
        assert_eq!(
            parse_quoted_string::<()>(r#"'hello world""#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_quoted_string::<()>(r#""hello world'"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(parse_quoted_string::<()>(r#"'""#), Err(nom::Err::Error(())));
        assert_eq!(parse_quoted_string::<()>(r#""'"#), Err(nom::Err::Error(())));
    }

    #[test]
    fn invalid_unquoted_string() {
        // unquoted strings can't contain quotes
        assert_eq!(parse_unquoted_string::<()>("''"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_unquoted_string::<()>(r#""""#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>("'hello world'"),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>(r#""hello world""#),
            Err(nom::Err::Error(()))
        );

        // this syntax is reserved for the double parser
        assert_eq!(
            parse_unquoted_string::<()>(r#"\$nan"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>(r#"\$inf"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>(r#"\$infinity"#),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>(r#"\$blahblah"#),
            Err(nom::Err::Error(()))
        );
    }

    #[test]
    fn null_characters() {
        // null characters are not allowed and lead to errors
        assert_eq!(parse_string::<()>("\0"), Err(nom::Err::Error(())));

        assert_eq!(parse_unquoted_string::<()>("\0"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_unquoted_string::<()>("\0hello"),
            Err(nom::Err::Error(()))
        );
        assert_eq!(
            parse_unquoted_string::<()>("\0hello world"),
            Err(nom::Err::Error(()))
        );

        assert_eq!(
            parse_quoted_string::<()>("\"\0\""),
            Err(nom::Err::Error(()))
        );
        assert_eq!(parse_quoted_string::<()>("'\0'"), Err(nom::Err::Error(())));
        assert_eq!(
            parse_quoted_string::<()>("'\0hello world'"),
            Err(nom::Err::Error(()))
        );
    }

    #[test]
    fn double_quoted_string() {
        assert_eq_quoted!(r#""hello world""#, "hello world", "");
        assert_eq_quoted!(r#""hello" world""#, "hello", r#" world""#);
        assert_eq_quoted!(r#""hello world" blah..."#, "hello world", " blah...");
        assert_eq_quoted!(r#""hello world"""#, "hello world", r#"""#);
        assert_eq_quoted!(r#""hello\" world""#, r#"hello" world"#, "");
        assert_eq_quoted!(r#""hello' world""#, "hello' world", "");
    }

    #[test]
    fn single_quoted_string() {
        assert_eq_quoted!("'hello world'", "hello world", "");
        assert_eq_quoted!("'hello' world'", "hello", " world'");
        assert_eq_quoted!("'hello world' blah...", "hello world", " blah...");
        assert_eq_quoted!("'hello world''", "hello world", "'");
        assert_eq_quoted!(r#"'hello\' world'"#, r#"hello' world"#, "");
        assert_eq_quoted!(r#"'hello" world'"#, r#"hello" world"#, "");
    }

    #[test]
    fn unquoted_string() {
        assert_eq_unquoted!("hello", "hello", "");
        assert_eq_unquoted!("hello world", "hello", " world");
        assert_eq_unquoted!("hello'world", "hello", "'world");
        assert_eq_unquoted!(r#"hello"world"#, "hello", r#""world"#);

        // whitespace characters are not included
        assert_eq_unquoted!("hello\tworld", "hello", "\tworld");
        assert_eq_unquoted!("hello\nworld", "hello", "\nworld");

        // escaped whitespace characters are included
        assert_eq_unquoted!(r#"hello\nworld"#, "hello\nworld", "");
        assert_eq_unquoted!(r#"hello\tworld"#, "hello\tworld", "");
        assert_eq_unquoted!(r#"\n"#, "\n", "");
        assert_eq_unquoted!(r#"\t"#, "\t", "");

        // non-latin characters and non-ascii characters are allowed
        assert_eq_unquoted!("שלום", "שלום", "");
        assert_eq_unquoted!("שלום עולם", "שלום", " עולם");
        assert_eq_unquoted!("håndter dette!", "håndter", " dette!");
    }
}
