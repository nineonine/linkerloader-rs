use crate::types::segment;
use crate::types::errors::ParseError;

pub struct ObjectFile {
    nsegs: Option<i32>,
    nsyms: Option<i32>,
    nrels: Option<i32>,
}

pub fn parse_object_file(input: &str) -> Result<ObjectFile, ParseError> {

    Err(ParseError::ParseError(String::from("Parsing failed")))
}
