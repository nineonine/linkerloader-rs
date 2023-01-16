use crate::types::errors::ParseError;
use crate::types::segment::{SegmentName, Segment};
use crate::types::symbol_table::{STE};

// Relocations, example:
//   loc seg ref type ...
// Loc is the location to be relocated, seg is the segment within which the
// location is found, ref is the segment or symbol number to be relocated there,
// and type is an architecture-dependent relocation type. Common types are
// A4 for a four-byte absolute address, or R4 for a four-byte relative address.
// Some relocation types may have extra fields after the type.
// Following the relocations comes the object data. The data for each segment
// is a single long hex string followed by a newline. (This makes it
// easy to read and write section data in perl.) Each pair of hex digits
// represents one byte. The segment data strings are in the same order as
// the segment table, and there must be segment data for each "present" segment.
// The length of the hex string is determined by the the defined length of the
#[derive(Debug)]
pub struct Relocation {
    pub rel_loc: i32, // location offset
    pub rel_seg: SegmentName,
    pub rel_ref: RelRef,
    pub rel_type: RelType,
}

#[derive(Debug)]
pub enum RelRef {
    SegmentRef(i32),
    SymbolRef(i32),
}

#[derive(Debug)]
pub enum RelType {
    A(i32),
    R(i32),
}

pub fn parse_relocation(segs: Vec<Segment>, st: Vec<STE>, s: &str) -> Result<Relocation, ParseError> {
    let rel_loc;
    let rel_seg;
    let rel_ref;
    let rel_type;

    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [loc, seg, _ref, ty] => {
            match i32::from_str_radix(loc, 16) {
                Err(_) => return Err(ParseError::InvalidRelLoc),
                Ok(i) => rel_loc = i,
            }
            match i32::from_str_radix(seg, 16) {
                Err(_) => return Err(ParseError::InvalidRelLoc),
                Ok(i) => {
                    match segs.get((i-1) as usize) {
                        None => return Err(ParseError::RelSegmentOutOfRange),
                        Some(s) => rel_seg = s.segment_name.clone(),
                    }
                },
            }
            // for now just always assume relocation refs are symbols
            match i32::from_str_radix(_ref, 16) {
                Err(_) => return Err(ParseError::InvalidRelLoc),
                Ok(i) => {
                    match st.get((i-1) as usize) {
                        None => return Err(ParseError::RelSymbolOutOfRange),
                        Some(_) => rel_ref = RelRef::SymbolRef(i),
                    }
                },
            }
            if let Some(c) = ty.chars().next() {
                match c {
                    'R' => match ty[1..].parse() {
                        Ok(i) => rel_type = RelType::R(i),
                        Err(_) => return Err(ParseError::InvalidRelType),
                    },
                    'A' => match ty[1..].parse() {
                        Ok(i) => rel_type = RelType::A(i),
                        Err(_) => return Err(ParseError::InvalidRelType),
                    },
                    _ => return Err(ParseError::InvalidRelType)
                }
            } else {
                return Err(ParseError::InvalidRelType);
            }

        },
        _otherwise => return Err(ParseError::InvalidRelocationEntry)
    }

    Ok(Relocation{
        rel_loc,
        rel_seg,
        rel_ref,
        rel_type,
    })
}
