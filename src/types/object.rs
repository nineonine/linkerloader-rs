use std::num::ParseIntError;

use crate::types::segment;
use crate::types::errors::ParseError;

#[derive(Debug)]
pub struct ObjectFile {
    nsegs: i32,
    nsyms: i32,
    nrels: i32,
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

    // nsegs nsyms nrels
    let nsegs: i32;
    let nsyms: i32;
    let nrels: i32;
    match input.next() {
        None => return Err(ParseError::MissingNSegsNSumsNRels),
        Some(vals) => {
            let vs: Vec<Result<i32, ParseIntError>> =
                    vals.split_whitespace()
                        .map(|x| x.parse())
                        .collect();
            match vs.as_slice() {
                [n_segs, n_syms, n_rels] => {
                    match *n_segs {
                        Ok(v) => nsegs = v,
                        Err(_) => return Err(ParseError::InvalidNSegsValue),
                    }
                    match *n_syms {
                        Ok(v) => nsyms = v,
                        Err(_) => return Err(ParseError::InvalidNSymsValue),
                    }
                    match *n_rels {
                        Ok(v) => nrels = v,
                        Err(_) => return Err(ParseError::InvalidNRelsValue),
                    }
                },
                _otherwise => return Err(ParseError::InvalidNSegsNSumsNRels)
            }
        }
    }

    return Ok(ObjectFile {
        nsegs,
        nsyms,
        nrels,
    });
}
