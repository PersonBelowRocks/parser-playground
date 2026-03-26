use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped_transform, tag, tag_no_case, take_while},
    character::complete::one_of,
    combinator::map_res,
    error::{FromExternalError, ParseError},
};
use std::str::FromStr;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid value")]
    Whatever,
}
