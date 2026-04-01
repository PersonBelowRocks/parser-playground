use std::cmp::min;

use miette::{Diagnostic, SourceOffset, SourceSpan};
use thiserror::Error;
use winnow::error::{ContextError, ParseError};

/// An error parsing an SKV key.
#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
#[error("error parsing key")]
#[diagnostic(code(error::parse::key), help("see docs for key syntax"))]
pub struct KeyParseError {
    pub message: String,
    #[source_code]
    pub input: String,
    #[label("{message}")]
    pub span: SourceSpan,
}

/// An error parsing an SKV map.
#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
#[error("error parsing map")]
#[diagnostic(code(error::parse::map), help("see docs for syntax"))]
pub struct MapParseError {
    pub message: String,
    #[source_code]
    pub input: String,
    #[label("{message}")]
    pub span: SourceSpan,
}

impl ErrorFromParts for KeyParseError {
    fn from_parts(message: String, input: String, span: SourceSpan) -> Self {
        Self {
            message,
            input,
            span,
        }
    }
}

impl ErrorFromParts for MapParseError {
    fn from_parts(message: String, input: String, span: SourceSpan) -> Self {
        Self {
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
