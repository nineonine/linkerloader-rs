use std::num::ParseIntError;

use crate::types::segment::{Segment, SegmentData, parse_segment, parse_segment_data};
use crate::types::symbol_table::{STE, parse_symbol_table_entry};
use crate::types::relocation::{Relocation, parse_relocation};
use crate::types::errors::ParseError;

#[derive(Debug)]
pub struct ObjectFile {
    pub nsegs: i32,
    pub nsyms: i32,
    pub nrels: i32,
    pub segments: Vec<Segment>,
    pub symbol_table: Vec<STE>,
    pub relocations: Vec<Relocation>,
    pub object_data: Vec<SegmentData>,
}

pub const MAGIC_NUMBER: &'static str = "LINK";

pub fn parse_object_file(file_contents: String) -> Result<ObjectFile, ParseError> {

    let mut input: std::iter::Peekable<std::str::Lines> = file_contents.lines().peekable();

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
                    match n_segs {
                        Ok(v) => nsegs = *v,
                        Err(_) => return Err(ParseError::InvalidNSegsValue),
                    }
                    match n_syms {
                        Ok(v) => nsyms = *v,
                        Err(_) => return Err(ParseError::InvalidNSymsValue),
                    }
                    match n_rels {
                        Ok(v) => nrels = *v,
                        Err(_) => return Err(ParseError::InvalidNRelsValue),
                    }
                },
                _otherwise => return Err(ParseError::InvalidNSegsNSumsNRels)
            }
        }
    }

    // parse segments
    let mut segs: Vec<Segment> = vec![];
    for _ in 0..nsegs {
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

    // parse symbol table
    let mut stes: Vec<STE> = vec![];
    for _ in 0..nsyms {
        match input.next() {
            Some(s) => {
                match parse_symbol_table_entry(nsegs, s) {
                    Ok(ste) => stes.push(ste),
                    Err(e) => return Err(e),
                }
            },
            None => return Err(ParseError::InvalidNumOfSTEs),
        }
    }
    let symbol_table: Vec<STE> = stes;
    // more segments than nsegs - error out
    if let Some(&l) = input.peek() {
        if parse_symbol_table_entry(nsegs, l).is_ok() {
            return Err(ParseError::InvalidNumOfSTEs);
        }
    }

    // parse relocation
    let mut rels: Vec<Relocation> = vec![];
    for _ in 0..nrels {
        match input.next() {
            Some(s) => {
                match parse_relocation(&segments, &symbol_table, s) {
                    Ok(rel) => rels.push(rel),
                    Err(e) => return Err(e),
                }
            },
            None => return Err(ParseError::InvalidNumOfRelocations),
        }
    }
    let relocations: Vec<Relocation> = rels;
    // more relocs than nrels - error out
    if let Some(&l) = input.peek() {
        if parse_relocation(&segments, &symbol_table, l).is_ok() {
            return Err(ParseError::InvalidNumOfRelocations);
        }
    }

    // parse object_data
    let mut seg_data: Vec<SegmentData> = vec![];
    for i in 0..nsegs {
        match input.next() {
            Some(s) => {
                println!("{:?}", segments[i as usize]);
                let seg_len = segments[i as usize].segment_len as usize;
                match parse_segment_data(seg_len, s) {
                    Ok(sd) => seg_data.push(sd),
                    Err(e) => return Err(e),
                }
            },
            None => return Err(ParseError::InvalidObjectData),
        }
    }
    let object_data: Vec<SegmentData> = seg_data;
    // more data than nsegs - error out
    if let Some(_) = input.next() {
        return Err(ParseError::SegmentDataOutOfBounds);
    }

    return Ok(ObjectFile {
        nsegs,
        nsyms,
        nrels,
        segments,
        symbol_table,
        relocations,
        object_data,
    });
}
