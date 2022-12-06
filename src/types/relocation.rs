use crate::types::segment::Segment;

pub struct Relocation {
    rel_loc: i32,
    rel_seg: Segment
}
