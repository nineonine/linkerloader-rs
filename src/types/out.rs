use std::collections::BTreeMap;
use std::ops::Deref;

use crate::types::object::MAGIC_NUMBER;
use crate::types::segment::*;

#[derive(Debug)]
pub struct ObjectOut {
    pub nsegs: i32,
    pub nsyms: i32,
    pub nrels: i32,
    pub segments: BTreeMap<SegmentName, Segment>,
    pub object_data: BTreeMap<SegmentName, SegmentData>,
}

impl Default for ObjectOut {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectOut {
    pub fn new() -> ObjectOut {
        ObjectOut {
            nsegs: 0,
            nsyms: 0,
            nrels: 0,
            segments: BTreeMap::new(),
            object_data: BTreeMap::new(),
        }
    }

    pub fn ppr(&self) -> String {
        let mut s = String::new();
        s.push_str(MAGIC_NUMBER);
        s.push_str("-OUT\n");
        let segment_order = vec![SegmentName::TEXT, SegmentName::DATA, SegmentName::BSS];
        s.push_str(format!("{:X} {:X} {:X}\n", self.nsegs, self.nsyms, self.nrels).as_str());
        let mut segs = vec![];
        let mut code_and_data = vec![];
        for segment_name in segment_order.iter() {
            if let Some(seg) = self.segments.get(segment_name) {
                let descrs = seg.ppr_seg_descr();
                segs.push(format!(
                    "{} {:X} {:X} {descrs}",
                    segment_name, seg.segment_start, seg.segment_len
                ));
                if let Some(segment_data) = self.object_data.get(segment_name) {
                    code_and_data.push(format!(
                        "  Obj code/data len: {:X} {}",
                        segment_data.len(),
                        segment_name
                    ));
                    let mut ppr_data = vec![];
                    for d in segment_data.deref().iter() {
                        ppr_data.push(format!("{d:02X}"));
                    }
                    code_and_data.push(ppr_data.join(" "));
                }
            }
        }
        s.push_str(segs.join("\n").as_str());
        s.push('\n');

        s.push_str(code_and_data.join("\n").as_str());
        s
    }
}
