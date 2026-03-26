/// Parsers for primitive SKV types.
mod boolean;
mod double;
mod integer;
mod string;

pub(crate) use boolean::parse_boolean;
pub(crate) use double::parse_double;
pub(crate) use integer::parse_integer;
pub(crate) use string::parse_string;
