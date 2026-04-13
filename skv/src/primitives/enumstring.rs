use std::str::FromStr;

use skv_core::EnumString;
use winnow::{error::ContextError, prelude::*, token::take_while};

#[inline(always)]
pub(crate) fn enum_string<'a, F: Fn(&EnumString) -> bool>(
    validate: F,
) -> impl ModalParser<&'a str, EnumString, ContextError> {
    take_while(1.., |ch: char| ch.is_alphanumeric() || ch == '_')
        .try_map(EnumString::from_str)
        .verify(validate)
}
