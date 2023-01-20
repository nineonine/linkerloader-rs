use crate::types::segment::Segment;

#[derive(Debug)]
pub struct ObjectOut {
    pub nsegs: i32,
    pub nsyms: i32,
    pub nrels: i32,
    pub segments: Vec<Segment>,
}

impl ObjectOut {
    pub fn new() -> ObjectOut {
        ObjectOut {
            nsegs: 0
          , nsyms: 0
          , nrels: 0
          , segments: vec![]
        }
    }
}
