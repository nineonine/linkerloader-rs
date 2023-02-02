use std::iter::Peekable;
use std::num::ParseIntError;
use std::ops::Deref;
use std::str::Lines;

use crate::types::errors::ParseError;
use crate::types::relocation::{parse_relocation, Relocation};
use crate::types::segment::{parse_segment, parse_segment_data, Segment, SegmentData};
use crate::types::symbol_table::{parse_symbol_table_entry, SymbolTableEntry};

#[derive(Debug)]
pub struct ObjectIn {
    pub nsegs: i32,
    pub nsyms: i32,
    pub nrels: i32,
    pub segments: Vec<Segment>,
    pub symbol_table: Vec<SymbolTableEntry>,
    pub relocations: Vec<Relocation>,
    // Following the relocations comes the object data. The data for each segment
    // is a single long hex string followed by a newline. (This makes it
    // easy to read and write section data in perl.) Each pair of hex digits
    // represents one byte. The segment data strings are in the same order as
    // the segment table, and there must be segment data for each "present" segment.
    // The length of the hex string is determined by the the defined length of the
    pub object_data: Vec<SegmentData>,
}

pub const MAGIC_NUMBER: &str = "LINK";

impl ObjectIn {
    pub fn ppr(&self, include_hdr: bool) -> String {
        let mut s = String::new();
        if include_hdr {
            s.push_str(MAGIC_NUMBER);
        }
        s.push_str(format!("{:X} {:X} {:X}\n", self.nsegs, self.nsyms, self.nrels).as_str());
        let mut segs = vec![];
        for seg in self.segments.iter() {
            let descrs = seg.ppr_seg_descr();
            segs.push(format!(
                "{} {:X} {:X} {descrs}",
                seg.segment_name, seg.segment_start, seg.segment_len
            ))
        }
        s.push_str(segs.join("\n").as_str());
        s.push('\n');

        let mut stes = vec![];
        for ste in self.symbol_table.iter() {
            stes.push(format!(
                "{} {:X} {:X} {}",
                ste.st_name, ste.st_value, ste.st_seg, ste.st_type
            ))
        }
        s.push_str(stes.join("\n").as_str());
        s.push('\n');

        let mut rels = vec![];
        for rel in self.relocations.iter() {
            let seg = self
                .segments
                .iter()
                .position(|s| s.segment_name == rel.rel_seg)
                .unwrap()
                + 1;
            rels.push(format!(
                "{:X} {:X} {} {}",
                rel.rel_loc, seg, rel.rel_ref, rel.rel_type
            ));
        }
        s.push_str(rels.join("\n").as_str());

        let mut code_data = vec![];
        for data in self.object_data.iter() {
            let mut ppr_data = vec![];
            for d in data.deref().iter() {
                ppr_data.push(format!("{d:02X}"));
            }
            code_data.push(ppr_data.join(" "));
        }
        s.push_str(code_data.join("\n").as_str());
        s
    }
}

pub fn parse_object_file(file_contents: String) -> Result<ObjectIn, ParseError> {
    let mut input: Peekable<Lines> = file_contents.lines().peekable();

    // magic number check
    match input.next() {
        None => return Err(ParseError::MissingMagicNumber),
        Some(mn) => {
            if mn != MAGIC_NUMBER {
                return Err(ParseError::InvalidMagicNumber);
            } else {
            }
        }
    }

    // nsegs nsyms nrels
    let nsegs: i32;
    let nsyms: i32;
    let nrels: i32;
    match parse_nsegs_nsyms_nrels(&mut input) {
        Err(e) => return Err(e),
        Ok((segs, syms, rels)) => {
            nsegs = segs;
            nsyms = syms;
            nrels = rels;
        }
    }

    // parse segments
    let mut segs: Vec<Segment> = vec![];
    for _ in 0..nsegs {
        match input.next() {
            Some(s) => match parse_segment(s) {
                Ok(seg) => segs.push(seg),
                Err(e) => return Err(e),
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
    let mut stes: Vec<SymbolTableEntry> = vec![];
    for _ in 0..nsyms {
        match input.next() {
            Some(s) => match parse_symbol_table_entry(nsegs, s) {
                Ok(ste) => stes.push(ste),
                Err(e) => return Err(e),
            },
            None => return Err(ParseError::InvalidNumOfSTEs),
        }
    }
    let symbol_table: Vec<SymbolTableEntry> = stes;
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
            Some(s) => match parse_relocation(&segments, &symbol_table, s) {
                Ok(rel) => rels.push(rel),
                Err(e) => return Err(e),
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
                // println!("{:?}", segments[i as usize]);
                let seg_len = segments[i as usize].segment_len as usize;
                match parse_segment_data(seg_len, s) {
                    Ok(sd) => seg_data.push(sd),
                    Err(e) => return Err(e),
                }
            }
            None => return Err(ParseError::InvalidObjectData),
        }
    }
    let object_data: Vec<SegmentData> = seg_data;
    // more data than nsegs - error out
    if input.next().is_some() {
        return Err(ParseError::SegmentDataOutOfBounds);
    }

    Ok(ObjectIn {
        nsegs,
        nsyms,
        nrels,
        segments,
        symbol_table,
        relocations,
        object_data,
    })
}

fn parse_nsegs_nsyms_nrels(input: &mut Peekable<Lines>) -> Result<(i32, i32, i32), ParseError> {
    let nsegs: i32;
    let nsyms: i32;
    let nrels: i32;
    match input.next() {
        None => return Err(ParseError::MissingNSegsNSumsNRels),
        Some(vals) => {
            let vs: Vec<Result<i32, ParseIntError>> = vals
                .split_whitespace()
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
                }
                _otherwise => return Err(ParseError::InvalidNSegsNSumsNRels),
            }
        }
    }
    Ok((nsegs, nsyms, nrels))
}
