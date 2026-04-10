use std::{cmp::min, collections::HashMap};

use miette::{Diagnostic, SourceOffset, SourceSpan};
use thiserror::Error;
use winnow::error::{ContextError, ParseError};

use crate::{Key, ValueType};

/// An error parsing an SKV key.
#[derive(Error, Debug, Clone, PartialEq)]
#[error("error parsing key")]
pub struct KeyParseError;

/// An error parsing an SKV map.
#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
pub enum MapParseError {
    #[error("error parsing map")]
    #[diagnostic(code(error::parse::map), help("see docs for syntax"))]
    Parsing {
        message: String,
        #[source_code]
        input: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[diagnostic(
        code(error::parse::map::missing_required_keys),
        help("provide the required keys")
    )]
    #[error("missing required keys: {0:?}")]
    MissingRequiredKeys(HashMap<Key, ValueType>),
}

impl ErrorFromParts for MapParseError {
    fn from_parts(message: String, input: String, span: SourceSpan) -> Self {
        Self::Parsing {
            message,
            input,
            span,
        }
    }
}

pub(crate) trait ErrorFromParts {
    fn from_parts(message: String, input: String, span: SourceSpan) -> Self;

    fn from_parse_error(error: ParseError<&str, ContextError>) -> Self
    where
        Self: Sized,
    {
        let span = error.char_span();
        let start = SourceOffset::from(span.start);
        let length = min(1, span.end.saturating_sub(span.start));

        let mut input = error.input().to_string();
        // otherwise miette doesn't point to anything in the error message in some cases
        input.push(' ');

        let message = match error.into_inner().to_string().as_str() {
            "" => "error".to_string(),
            msg => msg.to_string(),
        };

        Self::from_parts(message, input, SourceSpan::new(start, length))
    }
}

/// Error produced by operations on a parsed SKV map.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum MapError {
    /// Accessing a key that isn't present in the map.
    #[error("key not found")]
    NotFound,
    /// Accessing a value of a specific type but the value present in the map is of a different type.
    #[error("expected value of type '{expected:?}' but found '{found:?}'")]
    WrongType {
        expected: ValueType,
        found: ValueType,
    },
}
