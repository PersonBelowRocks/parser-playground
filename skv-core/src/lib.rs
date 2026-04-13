use std::{collections::HashSet, str::FromStr};

use ascii::{AsciiChar, AsciiStr, AsciiString};
use thiserror::Error;

/// Implemented by enums that can be used as values in an SKV schema.
/// This trait should be derived to ensure safety.
///
/// # Safety
/// The [`Self::ENUM_STRINGS`] constant must only contain strings that are valid [`EnumString`]s.
pub unsafe trait SkvEnum: Sized {
    fn enum_strings() -> HashSet<EnumString> {
        HashSet::from_iter(
            Self::ENUM_STRINGS
                .iter()
                .map(|&s| s.parse::<EnumString>().unwrap()),
        )
    }

    const ENUM_STRINGS: &'static [&'static str];

    fn to_enum_string(&self) -> EnumString;
    fn from_enum_string(s: &EnumString) -> Option<Self>;
}

/// A non-empty ASCII string containing only lowercase alphanumeric characters and underscores.
///
/// Intended to prevent crazy enums being used in schemas.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Into, derive_more::AsRef)]
pub struct EnumString(AsciiString);

impl EnumString {
    /// Checks if the given ASCII character is a valid character in an [`EnumString`].
    #[inline]
    pub fn is_valid_char(ch: AsciiChar) -> bool {
        // is_lowercase() isn't the opposite of is_uppercase():
        // both methods return "false" on non-alpha characters (like numbers)
        (ch.is_alphanumeric() && !ch.is_uppercase()) || ch == '_'
    }
}

impl From<EnumString> for String {
    fn from(value: EnumString) -> Self {
        value.0.into()
    }
}

/// Produced when trying to create an [`EnumString`].
#[derive(Clone, Debug, PartialEq, Error)]
#[error(
    "invalid enum string, must be a non-empty ASCII string w/ only underscores or lowercase alphanum. chars"
)]
pub struct EnumStringError;

impl FromStr for EnumString {
    type Err = EnumStringError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ascii_str = AsciiStr::from_ascii(s).map_err(|_| EnumStringError)?;
        if ascii_str.chars().all(Self::is_valid_char) {
            Ok(Self(ascii_str.to_ascii_string()))
        } else {
            Err(EnumStringError)
        }
    }
}
