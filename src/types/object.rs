use std::num::ParseIntError;

use crate::types::segment::{Segment, parse_segment};
use crate::types::errors::ParseError;

#[derive(Debug)]
pub struct ObjectFile {
    pub nsegs: i32,
    pub nsyms: i32,
    pub nrels: i32,
    pub segments: Vec<Segment>
}

pub const MAGIC_NUMBER: &'static str = "LINK";

pub fn parse_object_file(file_contents: String) -> Result<ObjectFile, ParseError> {

    let mut input = file_contents.lines().peekable();

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
                        .map(|x| i32::from_str_radix(x, 16))
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

    // parse segments
    let mut segs = vec![];
    for i in 0..nsegs {
        match input.next() {
            Some(s) => {
                match parse_segment(s) {
                    Ok(seg) => segs.push(seg),
                    Err(e) => return Err(e),
                }
            },
            None => return Err(ParseError::InvalidNumOfSegments),
        }
    }
    let segments: Vec<Segment> = segs;
    // more segments than nsegs - error out
    if let Some(&l) = input.peek() {
        if parse_segment(l).is_ok() {
            return Err(ParseError::InvalidNumOfSegments);
        }
    }

    return Ok(ObjectFile {
        nsegs,
        nsyms,
        nrels,
        segments,
    });
}
