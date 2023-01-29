use std::collections::{BTreeMap, HashMap};

use crate::types::errors::LinkError;
use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;
use crate::types::segment::{Segment, SegmentName};
use crate::types::symbol_table::{SymbolName, SymbolTableEntry};
use crate::logger::*;
use crate::utils::find_seg_start;

type Defn = (ObjectID, usize);
type Refs = HashMap<ObjectID, usize>;

pub struct LinkerInfo {
    pub segment_mapping: BTreeMap<ObjectID, BTreeMap<SegmentName, i32>>,
    pub common_block_mapping: HashMap<SymbolName, i32>,
    pub symbol_tables: HashMap<ObjectID, Vec<SymbolTableEntry>>,
    pub global_symtable: HashMap<SymbolName, (Option<Defn>, Refs)>,
}

impl LinkerInfo {
    pub fn new() -> LinkerInfo {
        let segment_mapping = BTreeMap::new();
        let common_block_mapping = HashMap::new();
        let symbol_tables = HashMap::new();
        let global_symtable = HashMap::new();
        LinkerInfo {
            segment_mapping,
            common_block_mapping,
            symbol_tables,
            global_symtable,
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
        let mut info = LinkerInfo::new();

        match self.allocate_storage(objects, &mut out, &mut info) {
            Err(e) => return Err(e),
            Ok(_) => {},
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

        // Implement Unix-style common blocks. That is, scan the symbol table for undefined symbols
        // with non-zero values, and add space of appropriate size to the .bss segment.
        let common_block = info.common_block_mapping.values().sum();
        if common_block != 0 {
            self.logger.debug(format!("Appending common block of size {:X} to BSS:", common_block).as_str());
            out.segments.entry(SegmentName::BSS)
                        .and_modify(|seg| {
                            seg.segment_len += common_block;
                        })
                        .or_insert_with(|| {
                            let mut seg = Segment::new(SegmentName::BSS);
                            seg.segment_start = bss_start;
                            seg.segment_len = common_block;
                            out.nsegs += 1;
                            seg
                        });
            self.logger.debug(format!("Object out (common block allocation):\n{}", out.ppr()).as_str());
        }


        /////////////////////////////////////////////
        self.logger.debug("Linking complete");
        self.logger.debug(format!("Object out (final):\n{}", out.ppr()).as_str());
        Ok((out, info))
    }

    fn allocate_storage(&mut self, objects: BTreeMap<ObjectID, ObjectIn>, out: &mut ObjectOut, info: &mut LinkerInfo) -> Result<(), LinkError> {
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

            // build symbol tables
            info.symbol_tables.insert(obj_id.to_string(), obj.symbol_table.clone());
            // global symtable updates
            for (i,symbol) in obj.symbol_table.iter().enumerate() {
                // skip common blocks!
                if symbol.is_common_block() {continue};
                // if symbol already defined in global table - error out
                if symbol.is_defined()
                    && info.global_symtable.contains_key(&symbol.st_name) {
                        return Err(LinkError::MultipleSymbolDefinitions)
                }
                info.global_symtable
                    .entry(symbol.st_name.to_string())
                    .and_modify(|(defn, refs)| {
                        if symbol.is_defined() {
                            assert!(defn.is_none());
                            *defn = Some((obj_id.to_string(), i));
                        } else {
                            refs.insert(obj_id.to_string(), i);
                        }
                    })
                    .or_insert_with(|| {
                        if symbol.is_defined() {
                            (Some((obj_id.to_string(), i)), HashMap::new())
                        } else {
                            let mut refs = HashMap::new();
                            refs.insert(obj_id.to_string(), i);
                            (None, refs)
                        }
                    });
            }

            info.segment_mapping.insert(obj_id.to_string(), seg_offsets);
            // common blocks
            for ste in obj.symbol_table.iter() {
                if ste.is_common_block() {
                    info.common_block_mapping.entry(ste.st_name.clone())
                            .and_modify({|size| {
                                if ste.st_value > *size {
                                    self.logger.debug(format!( "Adding comon block for symbol {} with size: {}"
                                                                  , ste.st_name, ste.st_value).as_str());
                                    *size = ste.st_value;
                                }
                            }})
                            .or_insert(ste.st_value);
                }
            }
        }
        Ok(())
    }
}
