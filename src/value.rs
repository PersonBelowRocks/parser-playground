use winnow::{
    combinator::{alt, cond},
    error::ContextError,
    prelude::*,
};

use crate::{
    error::MissingValueError,
    label,
    primitives::{parse_boolean, parse_double, parse_integer, parse_string},
    schema::SchemaValueType,
    util::token,
};

/// A value in an SKV map.
#[derive(Clone, Debug, PartialEq, strum::EnumDiscriminants, derive_more::From)]
#[strum_discriminants(name(ValueType))]
#[strum_discriminants(doc = "The type of an SKV value.")]
#[strum_discriminants(derive(
    strum::EnumIter,
    strum::VariantArray,
    strum::IntoStaticStr,
    strum::EnumString
))]
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
pub(crate) fn skv_value(input: &mut &str) -> ModalResult<Value> {
    schema_skv_value(None)
        .try_map(|o| o.ok_or(MissingValueError))
        .parse_next(input)
}

#[inline(always)]
fn accepts_type(schema: Option<&SchemaValueType>, value_type: ValueType) -> bool {
    schema.is_none_or(|t| t.value_type() == value_type)
}

#[inline(always)]
pub(crate) fn schema_skv_value(
    schema: Option<&SchemaValueType>,
) -> impl ModalParser<&str, Option<Value>, ContextError> {
    alt((
        // we do this termination logic to disambiguate between a boolean and an unquoted string that starts with "true" or "false".
        // we also can't consume the terminating whitespace since it could be a separator between this value and a key, so we use peek()
        cond(
            accepts_type(schema, ValueType::Bool),
            token(parse_boolean).map(Value::Bool),
        ),
        // same as with the boolean, we need to disambiguate
        cond(
            accepts_type(schema, ValueType::Int),
            token(parse_integer).map(Value::Int),
        ),
        cond(
            accepts_type(schema, ValueType::Double),
            token(parse_double).map(Value::Double),
        ),
        cond(
            accepts_type(schema, ValueType::String),
            parse_string.map(Value::String),
        ),
    ))
    .context(label("value"))
}

#[cfg(test)]
mod tests {
    use super::skv_value;
    use crate::Value;

    #[test]
    fn string_boolean_disambiguation() {
        // whitespace acts as a terminator for the boolean
        assert_eq!(skv_value(&mut "true "), Ok(Value::TRUE));
        assert_eq!(skv_value(&mut "true\n"), Ok(Value::TRUE));
        // this string has no whitespace, rather the newline is created during parsing, therefore it's treated as a string
        assert_eq!(skv_value(&mut r#""true\n""#), Ok(Value::string("true\n")));
        // these start with boolean values, but don't have any whitespace to terminate them
        assert_eq!(
            skv_value(&mut "truestring"),
            Ok(Value::string("truestring"))
        );
        assert_eq!(
            skv_value(&mut "falsestring"),
            Ok(Value::string("falsestring"))
        );
    }

    #[test]
    fn string_int_disambiguation() {
        assert_eq!(skv_value(&mut "150 test"), Ok(Value::int(150)));
        assert_eq!(skv_value(&mut "150test"), Ok(Value::string("150test")));
        assert_eq!(skv_value(&mut "150test next"), Ok(Value::string("150test")));
        assert_eq!(skv_value(&mut "0xff test"), Ok(Value::int(0xff)));
        assert_eq!(skv_value(&mut "0xfftest"), Ok(Value::string("0xfftest")));
    }

    #[test]
    fn string_double_disambiguation() {
        assert_eq!(skv_value(&mut "1.5 "), Ok(Value::double(1.5)));
        assert_eq!(skv_value(&mut "1.5"), Ok(Value::double(1.5)));
        assert_eq!(skv_value(&mut "1.5test"), Ok(Value::string("1.5test")));
        assert_eq!(skv_value(&mut "1.5 test"), Ok(Value::double(1.5)));
        assert_eq!(skv_value(&mut "1.5e12"), Ok(Value::double(1.5e12)));
    }

    #[test]
    fn valid_boolean_values() {
        assert_eq!(skv_value(&mut "true"), Ok(Value::TRUE));
        assert_eq!(skv_value(&mut "True"), Ok(Value::TRUE));
        assert_eq!(skv_value(&mut "false"), Ok(Value::FALSE));
        assert_eq!(skv_value(&mut "False"), Ok(Value::FALSE));
    }

    #[test]
    fn valid_string_values() {
        assert_eq!(skv_value(&mut "string!"), Ok(Value::string("string!")));
        assert_eq!(skv_value(&mut "space! "), Ok(Value::string("space!")));
        assert_eq!(
            skv_value(&mut r#""quoted string""#),
            Ok(Value::string("quoted string"))
        );
        assert_eq!(
            skv_value(&mut r#"'single quoted string'"#),
            Ok(Value::string("single quoted string"))
        );
        assert_eq!(
            skv_value(&mut r#""\u{af0f}""#),
            Ok(Value::string("\u{af0f}"))
        );
    }

    #[test]
    fn valid_integer_values() {
        assert_eq!(skv_value(&mut "150"), Ok(Value::int(150)));
        assert_eq!(skv_value(&mut "150 "), Ok(Value::int(150)));
        assert_eq!(skv_value(&mut "-66"), Ok(Value::int(-66)));
        assert_eq!(skv_value(&mut "0b11001100"), Ok(Value::int(0b11001100)));
    }

    #[test]
    fn valid_double_values() {
        assert_eq!(skv_value(&mut "1.5"), Ok(Value::double(1.5)));
        assert_eq!(skv_value(&mut "1.0"), Ok(Value::double(1.0)));
        assert_eq!(skv_value(&mut "-1.5"), Ok(Value::double(-1.5)));
        assert_eq!(skv_value(&mut "-1.0"), Ok(Value::double(-1.0)));

        assert_eq!(skv_value(&mut "1e1"), Ok(Value::double(1e1)));
        assert_eq!(skv_value(&mut "2e10"), Ok(Value::double(2e10)));
        assert_eq!(skv_value(&mut "1e-2"), Ok(Value::double(1e-2)));
        assert_eq!(skv_value(&mut "1.5e10"), Ok(Value::double(1.5e10)));
        assert_eq!(skv_value(&mut "1.5e-10"), Ok(Value::double(1.5e-10)));
        assert_eq!(skv_value(&mut "2E-10"), Ok(Value::double(2e-10)));

        match skv_value(&mut "\\$nan") {
            Ok(Value::Double(d)) => assert!(d.is_nan()),
            v @ _ => panic!("{v:?}"),
        }

        match skv_value(&mut "\\$inf") {
            Ok(Value::Double(d)) => assert!(d.is_infinite()),
            v @ _ => panic!("{v:?}"),
        }

        match skv_value(&mut "\\$infinity") {
            Ok(Value::Double(d)) => assert!(d.is_infinite()),
            v @ _ => panic!("{v:?}"),
        }
    }

    #[test]
    fn invalid_value() {
        // value can't be empty or whitespace
        assert!(skv_value(&mut "").is_err());
        assert!(skv_value(&mut " ").is_err());
        assert!(skv_value(&mut "\n").is_err());
        assert!(skv_value(&mut "\t").is_err());

        // broken scientific notation
        assert!(skv_value(&mut "1.5e").is_err());
        assert!(skv_value(&mut "1.5E").is_err());
        assert!(skv_value(&mut "1.5e").is_err());
        assert!(skv_value(&mut "1.5E5.5").is_err());
        assert!(skv_value(&mut "1.5E5.").is_err());
        assert!(skv_value(&mut "1.5E5a").is_err());
    }
}
