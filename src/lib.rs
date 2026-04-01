pub mod error;
mod key;
mod key_value_pair;
mod primitives;
mod skv;
mod value;

pub use key::Key;
pub use key_value_pair::KeyValuePair;
pub use skv::{Map, parse_map, parse_map_with_hasher};
pub use value::Value;
use winnow::error::{StrContext, StrContextValue};

pub(crate) fn expected(description: &'static str) -> StrContext {
    StrContext::Expected(StrContextValue::Description(description))
}

pub(crate) fn label(description: &'static str) -> StrContext {
    StrContext::Label(description)
}

/// Utilities for use in testing.
#[cfg(test)]
mod test_utils {
    use crate::Key;

    pub(crate) fn key(s: impl AsRef<str>) -> Key {
        Key::new(s.as_ref()).unwrap()
    }

    #[macro_export]
    macro_rules! kv {
        ($key:expr, $val:expr) => {
            crate::KeyValuePair::new(crate::test_utils::key($key), $val)
        };
    }
}
