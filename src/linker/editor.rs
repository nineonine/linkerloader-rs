use std::collections::{BTreeMap, HashMap};

use crate::logger::*;
use crate::types::errors::LinkError;
use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;
use crate::types::segment::{Segment, SegmentName};
use crate::types::symbol_table::{SymbolName, SymbolTableEntry};
use crate::utils::find_seg_start;

type Defn = (ObjectID, usize, Option<i32>);
type Refs = HashMap<ObjectID, usize>;
#[derive(Debug)]
pub struct LinkerInfo {
    pub segment_mapping: BTreeMap<ObjectID, BTreeMap<SegmentName, i32>>,
    pub common_block_mapping: HashMap<SymbolName, i32>,
    pub symbol_tables: HashMap<ObjectID, Vec<SymbolTableEntry>>,
    pub global_symtable: HashMap<SymbolName, (Option<Defn>, Refs)>,
}

impl Default for LinkerInfo {
    fn default() -> Self {
        Self::new()
    }
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
                    entry.push_str(format!(" {s_n}: {addr:X}").as_str());
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
        self.logger
            .debug(&format!("text_start: {:X}", self.text_start));
        self.logger.debug(&format!(
            "data_start_boundary: {:X}",
            self.data_start_boundary
        ));
        self.logger.debug(&format!(
            "bss_start_boundary: {:X}",
            self.bss_start_boundary
        ));
    }

    pub fn new(
        text_start: i32,
        data_start_boundary: i32,
        bss_start_boundary: i32,
        silent: bool,
    ) -> LinkerEditor {
        let mut r = LinkerEditor {
            text_start,
            data_start_boundary,
            bss_start_boundary,
            logger: Logger::new_stdout_logger(silent),
        };
        r.print_linker_editor_cfg();
        r
    }

    // for each object_in
    // for each segment in object_in
    //   * allocate storage in object_out
    //   * update address mappings in link_info
    pub fn link(
        &mut self,
        objects: BTreeMap<ObjectID, ObjectIn>,
    ) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        let mut out = ObjectOut::new();
        let mut info = LinkerInfo::new();

        self.alloc_storage_and_symtables(&objects, &mut out, &mut info)?;

        self.logger
            .debug(format!("Object out (initial allocation):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (initial allocation):\n{}", info.ppr()).as_str());

        // update TEXT start and patch segment addrs in link info
        self.patch_text_seg(&mut out, &mut info);

        // now do the same patching in DATA segment - update start and adjust address in info
        self.patch_data_seg(&mut out, &mut info);

        // now BSS. We might reuse the returned value in case we need to allocate common block
        let bss_start = self.patch_bss_seg(&mut out, &mut info);

        self.logger
            .debug(format!("Object out (segment offset patching):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (segment offset patching):\n{}", info.ppr()).as_str());

        // Implement Unix-style common blocks. That is, scan the symbol table for undefined symbols
        // with non-zero values, and add space of appropriate size to the .bss segment.
        self.common_block_allocation(&mut out, &mut info, bss_start);

        // resolve global symbols offsets
        self.resolve_global_sym_offsets(&objects, &mut info);

        /////////////////////////////////////////////
        self.logger.debug("Linking complete");
        self.logger
            .debug(format!("Object out (final):\n{}", out.ppr()).as_str());
        Ok((out, info))
    }

    // Allocate storage and build symbol tables
    fn alloc_storage_and_symtables(
        &mut self,
        objects: &BTreeMap<ObjectID, ObjectIn>,
        out: &mut ObjectOut,
        info: &mut LinkerInfo,
    ) -> Result<(), LinkError> {
        for (obj_id, obj) in objects.iter() {
            self.logger
                .debug(&format!("==> {}\n{}", obj_id, obj.ppr().as_str()));
            let mut seg_offsets = BTreeMap::new();
            for (i, segment) in obj.segments.iter().enumerate() {
                // allocate storage
                out.segments
                    .entry(segment.segment_name.clone())
                    .and_modify(|out_seg| {
                        let seg_offset = out_seg.segment_len;
                        seg_offsets.insert(segment.segment_name.clone(), seg_offset);
                        out_seg.segment_len = seg_offset + segment.segment_len;
                        self.logger.debug(&format!(
                            "new len for {}: 0x{:X} + 0x{:X} = 0x{:X}",
                            segment.segment_name,
                            seg_offset,
                            segment.segment_len,
                            out_seg.segment_len
                        ));
                    })
                    .or_insert_with(|| {
                        out.nsegs += 1;
                        seg_offsets.insert(segment.segment_name.clone(), 0);
                        let mut s = segment.clone();
                        s.segment_start = 0;
                        s
                    });
                // object data
                out.object_data
                    .entry(segment.segment_name.clone())
                    .and_modify(|segment_data| {
                        *segment_data = segment_data.concat(&obj.object_data[i]);
                    })
                    .or_insert_with(|| obj.object_data[i].clone());
            }

            // build symbol tables
            if let Some(err) = self.build_symbol_tables(info, obj, obj_id) {
                return Err(err);
            }

            info.segment_mapping.insert(obj_id.to_string(), seg_offsets);
            // common blocks
            for ste in obj.symbol_table.iter() {
                if ste.is_common_block() {
                    info.common_block_mapping
                        .entry(ste.st_name.clone())
                        .and_modify({
                            |size| {
                                if ste.st_value > *size {
                                    self.logger.debug(
                                        format!(
                                            "Adding comon block for symbol {} with size: {}",
                                            ste.st_name, ste.st_value
                                        )
                                        .as_str(),
                                    );
                                    *size = ste.st_value;
                                }
                            }
                        })
                        .or_insert(ste.st_value);
                }
            }
        }
        // Check for undefined symbols
        if info
            .global_symtable
            .values()
            .any(|(defn, _)| defn.is_none())
        {
            return Err(LinkError::UndefinedSymbolError);
        }

        Ok(())
    }

    fn build_symbol_tables(
        &mut self,
        info: &mut LinkerInfo,
        obj: &ObjectIn,
        obj_id: &str,
    ) -> Option<LinkError> {
        info.symbol_tables
            .insert(obj_id.to_string(), obj.symbol_table.clone());
        // global symtable updates
        for (i, symbol) in obj.symbol_table.iter().enumerate() {
            // skip common blocks!
            if symbol.is_common_block() {
                continue;
            };
            // if symbol already defined in global table - error out
            if symbol.is_defined()
                && info
                    .global_symtable
                    .get(&symbol.st_name)
                    .map_or(false, |x| x.0.is_some())
            {
                return Some(LinkError::MultipleSymbolDefinitions);
            }
            info.global_symtable
                .entry(symbol.st_name.to_string())
                .and_modify(|(defn, refs)| {
                    if symbol.is_defined() {
                        assert!(defn.is_none());
                        *defn = Some((obj_id.to_string(), i, None));
                    } else {
                        refs.insert(obj_id.to_string(), i);
                    }
                })
                .or_insert_with(|| {
                    if symbol.is_defined() {
                        (Some((obj_id.to_string(), i, None)), HashMap::new())
                    } else {
                        let mut refs = HashMap::new();
                        refs.insert(obj_id.to_string(), i);
                        (None, refs)
                    }
                });
        }
        None
    }

    fn patch_text_seg(&mut self, out: &mut ObjectOut, info: &mut LinkerInfo) {
        out.segments
            .entry(SegmentName::TEXT)
            .and_modify(|s| s.segment_start = self.text_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::TEXT).and_modify(|addr| {
                *addr += self.text_start;
            });
        }
    }

    fn patch_data_seg(&mut self, out: &mut ObjectOut, info: &mut LinkerInfo) {
        let text_end = out.segments.get(&SegmentName::TEXT).unwrap().segment_start
            + out.segments.get(&SegmentName::TEXT).unwrap().segment_len;
        let data_start = find_seg_start(text_end, self.data_start_boundary);
        out.segments
            .entry(SegmentName::DATA)
            .and_modify(|s| s.segment_start = data_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::DATA).and_modify(|addr| {
                *addr += data_start;
            });
        }
    }

    fn patch_bss_seg(&mut self, out: &mut ObjectOut, info: &mut LinkerInfo) -> i32 {
        let data_end = out.segments.get(&SegmentName::DATA).unwrap().segment_start
            + out.segments.get(&SegmentName::DATA).unwrap().segment_len;
        let bss_start = find_seg_start(data_end, self.bss_start_boundary);
        out.segments
            .entry(SegmentName::BSS)
            .and_modify(|s| s.segment_start = bss_start);
        for (_, addrs) in info.segment_mapping.iter_mut() {
            addrs.entry(SegmentName::BSS).and_modify(|addr| {
                *addr += bss_start;
            });
        }
        bss_start
    }

    fn common_block_allocation(
        &mut self,
        out: &mut ObjectOut,
        info: &mut LinkerInfo,
        bss_start: i32,
    ) {
        let common_block = info.common_block_mapping.values().sum();
        if common_block != 0 {
            self.logger
                .debug(format!("Appending common block of size {common_block:X} to BSS:").as_str());
            out.segments
                .entry(SegmentName::BSS)
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
            self.logger
                .debug(format!("Object out (common block allocation):\n{}", out.ppr()).as_str());
        }
    }

    // this assumes all definitions have been spotted and are in place
    fn resolve_global_sym_offsets(
        &self,
        objects: &BTreeMap<ObjectID, ObjectIn>,
        info: &mut LinkerInfo,
    ) {
        for (defn, _) in info.global_symtable.values_mut() {
            if let Some((mod_name, ste_i, addr)) = defn {
                let ste: &SymbolTableEntry = &info.symbol_tables.get(mod_name).unwrap()[*ste_i];
                assert!(ste.st_seg > 0);
                let seg_i = ste.st_seg as usize - 1;
                let sym_seg = &objects.get(mod_name).unwrap().segments[seg_i].segment_name;
                let segment_offset = *info
                    .segment_mapping
                    .get(mod_name)
                    .unwrap()
                    .get(sym_seg)
                    .unwrap();
                *addr = Some(segment_offset + ste.st_value);
            } else {
                panic!("resolve_global_sym_offsets: undefined symbol")
            }
        }
    }
}
