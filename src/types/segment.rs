use crate::types::symbol_table;
use crate::types::relocation;

pub struct Segment {
    segment_name: SegmentName,
    segment_start: i32,
    segment_descr: Vec<SegmentDescr>,
}

pub enum SegmentName {
    TEXT,
    DATA,
    BSS,
}

pub enum SegmentDescr {
    R, // readable
    W, // writable
    P, // oresebt in the object file
}
