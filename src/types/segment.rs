use std::collections::HashSet;
use crate::types::symbol_table;
use crate::types::relocation;

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
pub struct Segment {
    segment_name: SegmentName,
    segment_start: i32,
    segment_len: i32, // bytes
    segment_descr: HashSet<SegmentDescr>,
}

pub enum SegmentName {
    TEXT,
    DATA,
    BSS,
}

#[derive(Eq, PartialEq, Hash)]
pub enum SegmentDescr {
    R, // readable
    W, // writable
    P, // oresebt in the object file
}
