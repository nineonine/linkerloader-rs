use crate::types::segment::SegmentName;

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
pub struct Relocation {
    pub rel_loc: i32,
    pub rel_seg: SegmentName,
    pub rel_ref: RelRef,
    pub rel_type: RelType,
    pub rel_data: String,
}

pub enum RelRef {
    SegmentRef(i32),
    SymbolRef(i32),
}

pub enum RelType {
    A(i32),
    R(i32,)
}
