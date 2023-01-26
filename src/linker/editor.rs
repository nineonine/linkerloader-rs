use std::collections::BTreeMap;

use crate::types::errors::LinkError;
use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;
use crate::types::segment::{SegmentName};
use crate::logger::*;
use crate::utils::find_seg_start;

pub struct LinkerInfo {
    pub segment_mapping: BTreeMap<ObjectID, BTreeMap<SegmentName, i32>>
}

impl LinkerInfo {
    pub fn new(segment_mapping: BTreeMap<ObjectID, BTreeMap<SegmentName, i32>>) -> LinkerInfo {
        LinkerInfo {
            segment_mapping
        }
    }

    pub fn ppr(&self) -> String {
        let mut s = String::new();
        s.push_str("Link Info:\n");
        let mut es = vec![];
        let segment_order = vec![SegmentName::TEXT, SegmentName::DATA, SegmentName::BSS];
        for (obj_id, seg_addrs) in self.segment_mapping.iter() {
            let mut entry = String::new();
            entry.push_str(format!("  {} =>", &obj_id).as_str());
            for s_n in segment_order.iter() {
                if let Some(addr) = seg_addrs.get(s_n) {
                    entry.push_str(format!(" {}: {:X}", s_n, addr).as_str());
                }
            }
            es.push(entry);
        }
        s.push_str(es.join("\n").as_str());
        s
    }
}
pub struct LinkerEditor {
    text_start: i32,
    data_start_boundary: i32,
    bss_start_boundary: i32,
    logger: Logger,
}

type ObjectID = String;

impl LinkerEditor {
    fn print_linker_editor_cfg(&mut self) {
        self.logger.debug("Initializing Editor");
        self.logger.debug(&format!("text_start: {:X}", self.text_start));
        self.logger.debug(&format!("data_start_boundary: {:X}", self.data_start_boundary));
        self.logger.debug(&format!("bss_start_boundary: {:X}", self.bss_start_boundary));
    }

    pub fn new(text_start: i32, data_start_boundary: i32, bss_start_boundary: i32, silent: bool) -> LinkerEditor {
        let mut r = LinkerEditor {
            text_start,
            data_start_boundary,
            bss_start_boundary,
            logger: Logger::new_stdout_logger(silent),
        };
        r.print_linker_editor_cfg();
        return r;
    }
    // for each object_in
    // for each segment in object_in
    //   * allocate storage in object_out
    //   * update address mappings in link_info
    pub fn link(&mut self, objects: BTreeMap<ObjectID, ObjectIn>) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        let mut out = ObjectOut::new();
        let mut info = LinkerInfo::new(BTreeMap::new());
        for (obj_id, obj) in objects.iter() {
            self.logger.debug(&format!("==> {}\n{}", obj_id, obj.ppr().as_str()));
            let mut seg_offsets = BTreeMap::new();
            for (i,segment) in obj.segments.iter().enumerate() {
                // allocate storage
                out.segments.entry(segment.segment_name.clone())
                    .and_modify(|out_seg| {
                        let seg_offset = out_seg.segment_len;
                        seg_offsets.insert(segment.segment_name.clone(), seg_offset);
                        out_seg.segment_len = seg_offset + segment.segment_len;
                        self.logger.debug(&format!("new len for {}: 0x{:X} + 0x{:X} = 0x{:X}", segment.segment_name, seg_offset, segment.segment_len, out_seg.segment_len));

                    })
                    .or_insert_with(|| {
                        self.logger.debug(&format!("Segment {} not found. Adding", segment.segment_name));
                        out.nsegs = out.nsegs + 1;
                        seg_offsets.insert(segment.segment_name.clone(), 0);
                        let mut s = segment.clone();
                        s.segment_start = 0;
                        s
                    });
                // object data
                out.object_data.entry(segment.segment_name.clone())
                    .and_modify(|segment_data| {
                        *segment_data = segment_data.concat(&obj.object_data[i]);
                    })
                    .or_insert_with(|| {
                        return obj.object_data[i].clone();
                    });
            }
            info.segment_mapping.insert(obj_id.to_string(), seg_offsets);
        }

        self.logger.debug(format!("Object out (initial allocation):\n{}", out.ppr()).as_str());
        self.logger.debug(format!("Info (initial allocation):\n{}", info.ppr()).as_str());

        // update TEXT start and patch segment addrs in link info
        out.segments.entry(SegmentName::TEXT)
                    .and_modify(|s| s.segment_start = self.text_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::TEXT)
                 .and_modify(|addr| {
                    *addr = *addr + self.text_start;
                 });
        }

        // now do the same patching in DATA segment - update start and adjust address in info
        let text_end = out.segments.get(&SegmentName::TEXT).unwrap().segment_start
                          + out.segments.get(&SegmentName::TEXT).unwrap().segment_len;
        let data_start = find_seg_start(text_end, self.data_start_boundary);
        out.segments.entry(SegmentName::DATA)
                    .and_modify(|s| s.segment_start = data_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::DATA)
                 .and_modify(|addr| {
                    *addr = *addr + data_start;
                 });
        }

        // now BSS
        let data_end = out.segments.get(&SegmentName::DATA).unwrap().segment_start
                          + out.segments.get(&SegmentName::DATA).unwrap().segment_len;
        let bss_start = find_seg_start(data_end, self.bss_start_boundary);
        out.segments.entry(SegmentName::BSS)
                    .and_modify(|s| s.segment_start = bss_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::BSS)
                 .and_modify(|addr| {
                    *addr = *addr + bss_start;
                 });
        }
        self.logger.debug(format!("Object out (segment offset patching):\n{}", out.ppr()).as_str());
        self.logger.debug(format!("Info (segment offset patching):\n{}", info.ppr()).as_str());
        /////////////////////////////////////////////
        self.logger.debug("Linking complete");
        self.logger.debug(format!("Object out (final):\n{}", out.ppr()).as_str());
        Ok((out, info))
    }
}
