/// Opinionated string parsing code for both quoted and unquoted strings, with support for escape characters and unicode sequences.
/// Code is largely taken from the `nom` string example, with modifications to allow unquoted strings and a handful of other tweaks:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
///
/// # Notes on unicode sequences
/// Unicode sequences look like this: `\u{XXXX}`, where the `X` characters are hex digits. 1 to 6 hex digits are allowed between the braces.
/// All unicode characters are allowed and are escaped (converted into the actual characters in the output),
/// except for the null character (`\u{00}`), which is disallowed and will cause an error if included in the input string.
use winnow::{
    combinator::{Repeat, alt, delimited, preceded, repeat},
    error::{ContextError, ErrMode},
    prelude::*,
    token::{literal, take_while},
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
fn parse_unicode(input: &mut &str) -> ModalResult<char> {
    let parse_hex = take_while(1..=6, |c: char| c.is_ascii_hexdigit());
    let parse_delimited_hex = preceded(
        literal('u'),
        delimited(literal('{'), parse_hex, literal('}')),
    );

    let parse_u32 = parse_delimited_hex.try_map(|hex| u32::from_str_radix(hex, 16));

    parse_u32
        .try_map(char::try_from)
        .verify(|&c| c != '\0')
        .parse_next(input)
}

/// Parse an escaped character: \n, \t, \r, \u{00AC}, etc.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn parse_escaped_char(input: &mut &str) -> ModalResult<char> {
    preceded(
        literal('\\'),
        alt((
            alt((
                parse_unicode,
                'n'.value('\n'),
                'r'.value('\r'),
                't'.value('\t'),
                'b'.value('\u{08}'),
                'f'.value('\u{0C}'),
                '\\'.value('\\'),
                '/'.value('/'),
                '"'.value('"'),
            )),
            alt(('\''.value('\''),)),
        )),
    )
    .parse_next(input)
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
fn parse_literal<'a>(input: &mut &'a str, quote_type: QuoteType) -> ModalResult<&'a str> {
    let disallowed_characters = match quote_type {
        QuoteType::Single => ['\'', '\\', '\0'],
        QuoteType::Double => ['"', '\\', '\0'],
        QuoteType::Unquoted => {
            // If we're parsing an unquoted string, we also want to disallow whitespace.
            // `not` is a combinator that succeeds if its parser fails, and fails if
            // its parser succeeds. In this case, we want to ensure that the output of
            // not_quote_or_slash doesn't contain any whitespace, so we use not to check
            // that the input doesn't contain any whitespace.
            let parser = take_while(0.., |c: char| {
                !c.is_whitespace() && c != '"' && c != '\'' && c != '\\' && c != '\0'
            });

            return parser.verify(|s: &str| !s.is_empty()).parse_next(input);
        }
    };

    let not_quote_or_slash = take_while(0.., |c| !disallowed_characters.contains(&c));
    not_quote_or_slash
        .verify(|s: &str| !s.is_empty())
        .parse_next(input)
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
fn parse_fragment<'a>(
    input: &mut &'a str,
    quote_type: QuoteType,
) -> ModalResult<StringFragment<'a>> {
    alt((
        |i: &mut &'a str| parse_literal(i, quote_type).map(StringFragment::Literal),
        parse_escaped_char.map(StringFragment::EscapedChar),
    ))
    .parse_next(input)
}

/// Parser for an unquoted string.
///
/// This code is taken from the `nom` string example:
/// https://github.com/rust-bakery/nom/blob/main/examples/string.rs
#[inline(always)]
fn string_builder<'a>(
    quote_type: QuoteType,
) -> impl Parser<&'a str, String, ErrMode<ContextError>> {
    // fold is the equivalent of iterator::fold. It runs a parser in a loop,
    // and for each output value, calls a folding function on each output value.
    Repeat::fold(
        repeat(0.., move |i: &mut &'a str| parse_fragment(i, quote_type)),
        String::new,
        |mut string: String, fragment: StringFragment| {
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
pub(crate) fn parse_quoted_string(input: &mut &str) -> ModalResult<String> {
    // Finally, parse the string. Note that, if `build_string` could accept a raw
    // " character, the closing delimiter " would never match. When using
    // `delimited` with a looping parser (like fold), be sure that the
    // loop won't accidentally match your closing delimiter!
    if input.starts_with('\'') {
        delimited(
            literal('\''),
            string_builder(QuoteType::Single),
            literal('\''),
        )
        .parse_next(input)
    } else {
        delimited(
            literal('"'),
            string_builder(QuoteType::Double),
            literal('"'),
        )
        .parse_next(input)
    }
}

/// Parse an unquoted string.
///
/// Will produce an error if the string is empty, contains only whitespace, or contains a quote character (either `'` or `"`).
/// Escaped whitespace characters (e.g. `\t`, `\n`) are allowed will be escaped and included in the output string.
/// Whitespace is otherwise not allowed and will be treated as the end of the string.
#[allow(unused)]
#[inline(always)]
pub(crate) fn parse_unquoted_string(input: &mut &str) -> ModalResult<String> {
    string_builder(QuoteType::Unquoted)
        .verify(|s: &str| !s.is_empty())
        .parse_next(input)
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
pub(crate) fn parse_string(input: &mut &str) -> ModalResult<String> {
    alt((parse_quoted_string, parse_unquoted_string)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::{parse_quoted_string, parse_string, parse_unquoted_string};

    macro_rules! assert_eq_quoted {
        ($input:expr, $output:expr) => {
            assert_eq!(parse_quoted_string($input), Ok(str::to_string($output)));
        };
    }

    macro_rules! assert_eq_unquoted {
        ($input:expr, $output:expr) => {
            assert_eq!(parse_unquoted_string($input), Ok(str::to_string($output)));
        };
    }

    #[test]
    fn string() {
        assert_eq!(
            parse_string(&mut r#""hello world""#),
            Ok("hello world".to_string())
        );
        assert_eq!(
            parse_string(&mut "'hello world'"),
            Ok("hello world".to_string())
        );
        assert_eq!(parse_string(&mut "hello world"), Ok("hello".to_string()));
    }

    #[test]
    fn empty_string() {
        assert!(parse_string(&mut "").is_err());
        assert!(parse_string(&mut "   ").is_err());
        assert_eq!(parse_string(&mut "''"), Ok("".to_string()));
        assert_eq!(parse_string(&mut r#""""#), Ok("".to_string()));
    }

    #[test]
    fn invalid_string() {
        assert!(parse_string(&mut "'").is_err());
        assert!(parse_string(&mut r#"""#).is_err());
        assert!(parse_string(&mut "'hello world").is_err());
        assert!(parse_string(&mut r#""hello world"#).is_err());

        // this syntax is reserved for the double parser
        assert!(parse_string(&mut r#"\$nan"#).is_err());
        assert!(parse_string(&mut r#"\$inf"#).is_err());
        assert!(parse_string(&mut r#"\$infinity"#).is_err());
        assert!(parse_string(&mut r#"\$blahblah"#).is_err());
    }

    #[test]
    fn escape_characters() {
        // we don't care if broken syntax exists outside of the string
        assert_eq!(
            parse_string(&mut r#"hello \ø world"#),
            Ok("hello".to_string())
        );

        // unrecognized escape sequences cause an error
        assert!(parse_string(&mut r#"\ø"#).is_err());
        // broken unicode escape syntax causes an error
        assert!(parse_string(&mut r#"\u"#).is_err());
        assert!(parse_string(&mut r#"\u1234"#).is_err());
        assert!(parse_string(&mut r#"\u{1234"#).is_err());
        assert!(parse_string(&mut r#"\umeow"#).is_err());
        // null character is not allowed even when syntax is correct
        assert!(parse_string(&mut r#"\u{00}"#).is_err());

        assert_eq_unquoted!(&mut r#"\u{03A3}"#, "Σ");
        assert_eq_unquoted!(&mut r#"\u{6C34}"#, "水");
    }

    #[test]
    fn empty_quoted_string() {
        assert_eq_quoted!(&mut r#""""#, "");
        assert_eq_quoted!(&mut "''", "");
        assert_eq!(parse_quoted_string(&mut r#""""#), Ok("".to_string()));
        assert_eq!(parse_quoted_string(&mut "''"), Ok("".to_string()));
    }

    #[test]
    fn empty_unquoted_string() {
        // unquoted strings can't be empty or whitespace-only
        assert!(parse_unquoted_string(&mut "").is_err());
        assert!(parse_unquoted_string(&mut "   ").is_err());
    }

    #[test]
    fn invalid_quoted_string() {
        // quoted strings require quotes
        assert!(parse_quoted_string(&mut "").is_err());
        assert!(parse_quoted_string(&mut "   ").is_err());
        assert!(parse_quoted_string(&mut "hello").is_err());
        assert!(parse_quoted_string(&mut "hello world").is_err());

        // missing quotes cause an error
        // (double quotes)
        assert!(parse_quoted_string(&mut r#"""#).is_err());
        assert!(parse_quoted_string(&mut r#""hello world"#).is_err());
        assert!(parse_quoted_string(&mut r#"hello world""#).is_err());
        // (single quotes)
        assert!(parse_quoted_string(&mut "'").is_err());
        assert!(parse_quoted_string(&mut "'hello world").is_err());
        assert!(parse_quoted_string(&mut "hello world'").is_err());

        // mismatched quotes cause an error
        assert!(parse_quoted_string(&mut r#"'hello world""#).is_err());
        assert!(parse_quoted_string(&mut r#""hello world'"#).is_err());
        assert!(parse_quoted_string(&mut r#"'""#).is_err());
        assert!(parse_quoted_string(&mut r#""'"#).is_err());
    }

    #[test]
    fn invalid_unquoted_string() {
        // unquoted strings can't contain quotes
        assert!(parse_unquoted_string(&mut "''").is_err());
        assert!(parse_unquoted_string(&mut r#""""#).is_err());
        assert!(parse_unquoted_string(&mut "'hello world'").is_err());
        assert!(parse_unquoted_string(&mut r#""hello world""#).is_err());

        // this syntax is reserved for the double parser
        assert!(parse_unquoted_string(&mut r#"\$nan"#).is_err());
        assert!(parse_unquoted_string(&mut r#"\$inf"#).is_err());
        assert!(parse_unquoted_string(&mut r#"\$infinity"#).is_err());
        // any \$xxxx string should fail
        assert!(parse_unquoted_string(&mut r#"\$blahblah"#).is_err());
    }

    #[test]
    fn null_characters() {
        // null characters are not allowed and lead to errors
        assert!(parse_string(&mut "\0").is_err());

        assert!(parse_unquoted_string(&mut "\0").is_err());
        assert!(parse_unquoted_string(&mut "\0hello").is_err());
        assert!(parse_unquoted_string(&mut "\0hello world").is_err());

        assert!(parse_quoted_string(&mut "\"\0\"").is_err());
        assert!(parse_quoted_string(&mut "'\0'").is_err());
        assert!(parse_quoted_string(&mut "'\0hello world'").is_err());
    }

    #[test]
    fn double_quoted_string() {
        assert_eq_quoted!(&mut r#""hello world""#, "hello world");
        assert_eq_quoted!(&mut r#""hello" world""#, "hello");
        assert_eq_quoted!(&mut r#""hello world" blah..."#, "hello world");
        assert_eq_quoted!(&mut r#""hello world"""#, "hello world");
        assert_eq_quoted!(&mut r#""hello\" world""#, r#"hello" world"#);
        assert_eq_quoted!(&mut r#""hello' world""#, "hello' world");
    }

    #[test]
    fn single_quoted_string() {
        assert_eq_quoted!(&mut "'hello world'", "hello world");
        assert_eq_quoted!(&mut "'hello' world'", "hello");
        assert_eq_quoted!(&mut "'hello world' blah...", "hello world");
        assert_eq_quoted!(&mut "'hello world''", "hello world");
        assert_eq_quoted!(&mut r#"'hello\' world'"#, r#"hello' world"#);
        assert_eq_quoted!(&mut r#"'hello" world'"#, r#"hello" world"#);
    }

    #[test]
    fn unquoted_string() {
        assert_eq_unquoted!(&mut "hello", "hello");
        assert_eq_unquoted!(&mut "hello world", "hello");
        assert_eq_unquoted!(&mut "hello'world", "hello");
        assert_eq_unquoted!(&mut r#"hello"world"#, "hello");

        // whitespace characters are not included
        assert_eq_unquoted!(&mut "hello\tworld", "hello");
        assert_eq_unquoted!(&mut "hello\nworld", "hello");

        // escaped whitespace characters are included
        assert_eq_unquoted!(&mut r#"hello\nworld"#, "hello\nworld");
        assert_eq_unquoted!(&mut r#"hello\tworld"#, "hello\tworld");
        assert_eq_unquoted!(&mut r#"\n"#, "\n");
        assert_eq_unquoted!(&mut r#"\t"#, "\t");

        // non-latin characters and non-ascii characters are allowed
        assert_eq_unquoted!(&mut "שלום", "שלום");
        assert_eq_unquoted!(&mut "שלום עולם", "שלום");
        assert_eq_unquoted!(&mut "håndter dette!", "håndter");
    }
}
