use winnow::{
    ModalResult,
    combinator::{opt, terminated},
    error::ContextError,
    prelude::*,
    token::literal,
};

use crate::{Key, Value, key::skv_key, schema::Schema, value::skv_value};

/// An SKV key-value pair.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyValuePair {
    pub key: Key,
    pub value: Value,
}

impl KeyValuePair {
    /// Creates a new `KeyValuePair` from a key and a value.
    #[inline]
    #[must_use]
    pub fn new(key: Key, value: impl Into<Value>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }
}

impl From<KeyValuePair> for (Key, Value) {
    #[inline]
    fn from(pair: KeyValuePair) -> Self {
        (pair.key, pair.value)
    }
}

#[inline(always)]
pub(crate) fn skv_pair<'a: 'b, 'b>(
    schema: Option<&'a Schema>,
) -> impl ModalParser<&'b str, KeyValuePair, ContextError> + 'a {
    move |input: &mut &'b str| -> ModalResult<KeyValuePair> {
        let key = terminated(
            skv_key,
            (opt(literal(' ')), literal('='), opt(literal(' '))),
        )
        .parse_next(input)?;

        let schema_value = schema.and_then(|sch| sch.get_value(&key));
        skv_value(schema_value)
            .parse_next(input)
            .map(|value| KeyValuePair { key, value })
    }
}

#[cfg(test)]
mod tests {
    use winnow::{ModalResult, Parser};

    use crate::{Value, kv};

    use super::{KeyValuePair, skv_pair as schemaed_skv_pair};

    fn skv_pair(input: &mut &str) -> ModalResult<KeyValuePair> {
        schemaed_skv_pair(None).parse_next(input)
    }

    // needed so we can convert literals (static strings) to values in the tests.
    // but we don't want to do this elsewhere since it hides allocations.
    impl From<&str> for Value {
        fn from(s: &str) -> Self {
            Value::from(s.to_string())
        }
    }

    macro_rules! assert_kv_eq {
        ($input:expr, $expected:expr, $remaining:expr) => {
            let input_string = format!("{}{}", $input, $remaining);
            let mut input = input_string.as_str();
            let expected: KeyValuePair = $expected;

            let pair = skv_pair(&mut input).unwrap();

            match expected.value {
                Value::Double(ex) => {
                    // doubles (or floats in general) require special equality handling since NaN != NaN
                    if ex.is_nan() || ex.is_infinite() {
                        assert!(matches!(pair.value, Value::Double(d) if d.is_nan() == ex.is_nan()), "(pair double val eq test) input: {}", input);
                        assert!(matches!(pair.value, Value::Double(d) if d.is_infinite() == ex.is_infinite()), "(pair double val eq test) input: {}", input);
                        assert_eq!(pair.key, expected.key, "(pair key eq test) input: {}", input);
                    } else {
                        assert_eq!(pair, expected, "(pair eq test) input: {}", input);
                    }
                }
                _ => assert_eq!(pair, expected, "(pair eq test) input: {}", input)
            }

            assert_eq!(input, $remaining, "(remaining input test) input: {}", input);
        };
    }

    macro_rules! remaining_input_test {
        ($input:expr, $expected:expr) => {
            for remaining in ["", " ", " rest", " 123", "\n", "\nrest", "\twhitespace"] {
                assert_kv_eq!($input, $expected, remaining);
            }
        };
    }

    macro_rules! kv_pair_asserts {
        ($key:literal, $val:literal, $expected_val:expr) => {
            let inputs = [
                concat!($key, "=", $val),
                concat!($key, " = ", $val),
                concat!($key, " =", $val),
                concat!($key, "= ", $val),
            ];
            let expected = kv!($key, $expected_val);

            for input in inputs {
                remaining_input_test!(input, expected.clone());
            }
        };
    }

    macro_rules! skv_test_asserts {
        ($val:literal, $expected_val:expr) => {
            kv_pair_asserts!("test", $val, $expected_val);
            kv_pair_asserts!("test.parts", $val, $expected_val);
            kv_pair_asserts!("test.parts2", $val, $expected_val);
            kv_pair_asserts!("test1.parts2", $val, $expected_val);
            kv_pair_asserts!("test1.parts2.part3", $val, $expected_val);
        };
    }

    macro_rules! skv_test {
        ($name:ident, $val:literal, $expected_val:expr) => {
            #[test]
            fn $name() {
                skv_test_asserts!($val, $expected_val);
            }
        };
    }

    skv_test!(dec_integer_pair, "123", 123);
    skv_test!(hex_integer_pair, "0xffaa12", 0xffaa12);
    skv_test!(oct_integer_pair, "0o660", 0o660);
    skv_test!(bin_integer_pair, "0b1010", 0b1010);

    skv_test!(unquoted_str, "hello", "hello");
    skv_test!(
        unquoted_str_unicode,
        r#"unicode\u{ffa0}"#,
        "unicode\u{ffa0}"
    );
    skv_test!(unquoted_str_newline, r#"hello\nworld"#, "hello\nworld");
    skv_test!(unquoted_str_tab, r#"hello\tworld"#, "hello\tworld");

    skv_test!(quoted_str, r#""hello world""#, "hello world");
    skv_test!(
        quoted_str_unicode,
        r#""hello unicode: \u{ffff}""#,
        "hello unicode: \u{ffff}"
    );
    skv_test!(
        quoted_str_newline,
        r#""hello world\ngood day!""#,
        "hello world\ngood day!"
    );
    skv_test!(
        quoted_str_tab,
        r#""hello \t tab \t !""#,
        "hello \t tab \t !"
    );

    #[test]
    fn boolean() {
        skv_test_asserts!("true", true);
        skv_test_asserts!("True", true);
        skv_test_asserts!("TRUE", true);
        skv_test_asserts!("false", false);
        skv_test_asserts!("False", false);
        skv_test_asserts!("FALSE", false);
    }

    skv_test!(nan_double, r#"\$nan"#, { f64::NAN });
    skv_test!(inf_double, r#"\$inf"#, { f64::INFINITY });
    skv_test!(verbose_inf_double, r#"\$infinity"#, { f64::INFINITY });
    skv_test!(neg_inf_double, r#"-\$inf"#, { f64::NEG_INFINITY });
    skv_test!(verbose_neg_inf_double, r#"-\$infinity"#, {
        f64::NEG_INFINITY
    });

    #[test]
    fn double() {
        skv_test_asserts!("1.5", 1.5);
        skv_test_asserts!("-1.5", -1.5);
        skv_test_asserts!("1.5e10", 1.5e10);
        skv_test_asserts!("1.5e-10", 1.5e-10);
        skv_test_asserts!("2e10", 2e10);
        skv_test_asserts!("2e-10", 2e-10);
        skv_test_asserts!("-2e-10", -2e-10);
        skv_test_asserts!("2E-10", 2e-10);
    }
}
