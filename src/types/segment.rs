use crate::types::errors::ParseError;

// Each segment definition contains the
// segment name, the address where the segment logically starts, the length
// of the segment in bytes, and a string of code letters describing the segment.
// Code letters include R for readable, W for writable, and P for present in the
// object file. (Other letters may be present as well.) A typical set of segments
// for an a.out like file would be:
//   .text 1000 2500 RP
//   .data 4000 C00 RWP
//   .bss 5000 1900 RW
// Segments are numbered in the order their definitions appear, with the first
// segment being number 1.
#[derive(Debug)]
pub struct Segment {
    pub segment_name: SegmentName,
    pub segment_start: i32,
    pub segment_len: i32, // bytes
    pub segment_descr: Vec<SegmentDescr>, // TODO: ensure uniqueness when parsing
}

// TODO: allow any name?
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SegmentName {
    TEXT,
    DATA,
    BSS,
    CUSTOM(String),
}

#[derive(Debug, Eq, PartialEq)]
pub enum SegmentDescr {
    R, // readable
    W, // writable
    P, // oresebt in the object file
}

pub fn parse_segment(s: &str) -> Result<Segment, ParseError> {
    let segment_name;
    let segment_start;
    let segment_len;
    let segment_descr;
    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [name, start, len, descr] => {
            match *name {
                ".text" => { segment_name = SegmentName::TEXT },
                ".data" => { segment_name = SegmentName::DATA },
                ".bss" =>  { segment_name = SegmentName::BSS },
                s =>
                    if s.starts_with(".") { segment_name = SegmentName::CUSTOM(String::from(s)) }
                    else { return Err(ParseError::InvalidSegmentName); }
            }
            match i32::from_str_radix(start, 16) {
                Err(_) => return Err(ParseError::InvalidSegmentStart),
                Ok(i) => segment_start = i,
            }
            match i32::from_str_radix(len, 16) {
                Err(_) => return Err(ParseError::InvalidSegmentLen),
                Ok(i) => segment_len = i,
            }
            let mut descrs: Vec<SegmentDescr> = vec![];
            for c in descr.chars() {
                match segment_descr_from_chr(c) {
                    Err(e) => return Err(e),
                    Ok(sd) => descrs.push(sd),
                }
            }
            segment_descr = descrs;
        },
        _otherwise => return Err(ParseError::InvalidSegment)
    }

    return Ok(Segment {
        segment_name,
        segment_start,
        segment_len,
        segment_descr,
    })
}

fn segment_descr_from_chr(c: char) -> Result<SegmentDescr, ParseError> {
    match c {
        'R' => Ok(SegmentDescr::R),
        'W' => Ok(SegmentDescr::W),
        'P' => Ok(SegmentDescr::P),
        _   => Err(ParseError::InvalidSegmentDescr),
    }
}
