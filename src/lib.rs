mod key;
mod primitives;
mod value;

pub use key::Key;
use nom_locate::LocatedSpan;
pub use value::Value;

pub(crate) type Span<'a> = LocatedSpan<&'a str>;
