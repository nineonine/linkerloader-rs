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
// Some relocation types may have extra fields after the type. (TODO)
#[derive(Debug, Clone)]
pub struct Relocation {
    pub rel_loc: i32, // relocation address
    pub rel_seg: SegmentName,
    pub rel_ref: RelRef,
    pub rel_type: RelType,
}

#[derive(Debug, Eq, PartialEq, Clone)]
// Note that we store 0-based indexes !!!
pub enum RelRef {
    SegmentRef(usize),
    SymbolRef(usize),
    NoRef,
}

impl fmt::Display for RelRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rel_ref_str = match self {
            RelRef::SegmentRef(s) => format!("{s:X}"),
            RelRef::SymbolRef(s) => format!("{s:X}"),
            RelRef::NoRef => String::new(),
        };
        write!(f, "{rel_ref_str}")
    }
}

// * A4 Absolute reference. The four bytes at loc are an absolute reference to segment ref.
// * R4 Relative reference. The four bytes at loc are a relative reference to segment ref.
//   That is, the bytes at loc contain the difference between the address after loc (loc+4)
//   and the target address. (This is the x86 relative jump instruction format.)
// * AS4 Absolute symbol reference. The four bytes at loc are an absolute reference to symbol
//   ref, with the addend being the value already stored at loc. (The addend is usually zero.)
// * RS4 Relative symbol reference. The four bytes at loc are a relative reference to symbol ref,
//   with the addend being the value already stored at loc. (The addend is usually zero.)
// * U2 Upper half reference. The two bytes at loc are the most significant two bytes of a
//   reference to symbol ref.
// * L2 Lower half reference. The two bytes at loc are the least significant two bytes of a
//   reference to symbol ref.
// * GA4: (GOT address) At location loc, store the distance to the GOT.
// * GP4: (GOT pointer) Put a pointer to symbol ref in the GOT, and at
//   location loc, store the GOT-relative offset of that pointer.
// * GR4: (GOT relative) Location loc contains an address in segment
//   ref. Replace that with the offset from the beginning of the GOT to
//   that address.
// * ER4: (Executable relative) Location loc contains an address relative to the beginning of
//   the executable. The ref field is ignored.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum RelType {
    A4,
    R4,
    AS4,
    RS4,
    U2,
    L2,
    GA4,
    GP4,
    GR4,
    ER4,
}

impl RelType {
    fn is_segment_rel(&self) -> bool {
        matches!(self, RelType::A4 | RelType::R4 | RelType::GR4)
    }

    fn is_no_rel(&self) -> bool {
        matches!(self, RelType::GA4 | RelType::ER4)
    }
}

impl fmt::Display for RelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rel_type_str = match self {
            RelType::A4 => "A4".to_string(),
            RelType::R4 => "R4".to_string(),
            RelType::AS4 => "AS4".to_string(),
            RelType::RS4 => "RS4".to_string(),
            RelType::U2 => "U2".to_string(),
            RelType::L2 => "L2".to_string(),
            RelType::GA4 => "GA4".to_string(),
            RelType::GP4 => "GP4".to_string(),
            RelType::GR4 => "GR4".to_string(),
            RelType::ER4 => "ER4".to_string(),
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
            rel_type = match *ty {
                "A4" => RelType::A4,
                "R4" => RelType::R4,
                "AS4" => RelType::AS4,
                "RS4" => RelType::RS4,
                "U2" => RelType::U2,
                "L2" => RelType::L2,
                "GA4" => RelType::GA4,
                "GP4" => RelType::GP4,
                "GR4" => RelType::GR4,
                "ER4" => RelType::ER4,
                _ => return Err(ParseError::InvalidRelType),
            };
            match usize::from_str_radix(_ref, 16) {
                Err(_) => return Err(ParseError::InvalidRelRef),
                Ok(i) => {
                    if rel_type.is_segment_rel() {
                        match segs.get(i - 1) {
                            None => return Err(ParseError::RelSegmentOutOfRange),
                            Some(_) => rel_ref = RelRef::SegmentRef(i - 1),
                        }
                    } else if rel_type.is_no_rel() {
                        rel_ref = RelRef::NoRef;
                    } else {
                        match st.get(i - 1) {
                            None => return Err(ParseError::RelSymbolOutOfRange),
                            Some(_) => rel_ref = RelRef::SymbolRef(i - 1),
                        }
                    }
                }
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
