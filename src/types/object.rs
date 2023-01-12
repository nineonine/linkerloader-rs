use crate::types::segment;
use crate::types::errors::ParseError;

pub struct ObjectFile {
    nsegs: Option<i32>,
    nsyms: Option<i32>,
    nrels: Option<i32>,
}

pub const MAGIC_NUMBER: &'static str = "LINK";

pub fn parse_object_file(file_contents: String) -> Result<ObjectFile, ParseError> {

    let mut input = file_contents.lines();

    // magic number check
    match input.next() {
        None => return Err(ParseError::MissingMagicNumber),
        Some(mn) => {
            if mn != MAGIC_NUMBER {return Err(ParseError::InvalidMagicNumber)}
            else {}
        }
    }

    Err(ParseError::ParseError(String::from("Parsing failed")))
}
