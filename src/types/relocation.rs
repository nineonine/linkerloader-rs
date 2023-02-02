use std::fmt;

use crate::types::errors::ParseError;
use crate::types::segment::{Segment, SegmentName};
use crate::types::symbol_table::SymbolTableEntry;

// Relocations, example:
//   loc seg ref type ...
// Loc is the location to be relocated, seg is the segment within which the
// location is found, ref is the segment or symbol number to be relocated there,
// and type is an architecture-dependent relocation type. Common types are
// A4 for a four-byte absolute address, or R4 for a four-byte relative address.
// Some relocation types may have extra fields after the type.
#[derive(Debug)]
pub struct Relocation {
    pub rel_loc: i32, // relocation address
    pub rel_seg: SegmentName,
    pub rel_ref: RelRef,
    pub rel_type: RelType,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RelRef {
    SegmentRef(usize),
    SymbolRef(usize),
}

impl fmt::Display for RelRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rel_ref_str = match self {
            RelRef::SegmentRef(s) => format!("{s:X}"),
            RelRef::SymbolRef(s) => format!("{s:X}"),
        };
        write!(f, "{rel_ref_str}")
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RelType {
    A(i32),
    R(i32),
}

impl fmt::Display for RelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rel_type_str = match self {
            RelType::A(i) => format!("A{i:X}"),
            RelType::R(i) => format!("R{i:X}"),
        };
        write!(f, "{rel_type_str}")
    }
}

pub fn parse_relocation(
    segs: &[Segment],
    st: &[SymbolTableEntry],
    s: &str,
) -> Result<Relocation, ParseError> {
    let rel_loc;
    let rel_seg;
    let rel_ref;
    let rel_type;

    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [loc, seg, _ref, ty] => {
            match i32::from_str_radix(loc, 16) {
                Err(_) => return Err(ParseError::InvalidRelRef),
                Ok(i) => rel_loc = i,
            }
            match i32::from_str_radix(seg, 16) {
                Err(_) => return Err(ParseError::InvalidRelSegment),
                Ok(i) => match segs.get((i - 1) as usize) {
                    None => return Err(ParseError::RelSegmentOutOfRange),
                    Some(s) => rel_seg = s.segment_name.clone(),
                },
            }
            match usize::from_str_radix(_ref, 16) {
                Err(_) => return Err(ParseError::InvalidRelRef),
                Ok(i) => {
                    match st.get(i - 1) {
                        None => return Err(ParseError::RelSymbolOutOfRange),
                        // for now just always assume relocation refs are symbols
                        Some(_) => rel_ref = RelRef::SymbolRef(i),
                    }
                }
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
                    _ => return Err(ParseError::InvalidRelType),
                }
            } else {
                return Err(ParseError::InvalidRelType);
            }
        }
        _otherwise => return Err(ParseError::InvalidRelocationEntry),
    }

    Ok(Relocation {
        rel_loc,
        rel_seg,
        rel_ref,
        rel_type,
    })
}
