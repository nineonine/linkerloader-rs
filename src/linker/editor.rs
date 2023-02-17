use std::collections::{BTreeMap, HashMap, HashSet};

use crate::logger::*;
use crate::types::errors::LinkError;
use crate::types::library::StaticLib;
use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;
use crate::types::relocation::{RelRef, RelType};
use crate::types::segment::{Segment, SegmentName};
use crate::types::symbol_table::{SymbolName, SymbolTableEntry};
use crate::utils::{find_seg_start, mk_addr_4, mk_i_4, x_to_i2, x_to_i4};

type Defn = (ObjectID, usize, Option<i32>);
type Refs = HashMap<ObjectID, usize>;
#[derive(Debug)]
pub struct LinkerInfo {
    pub segment_mapping: BTreeMap<ObjectID, BTreeMap<SegmentName, i32>>,
    pub common_block_mapping: HashMap<SymbolName, i32>,
    pub symbol_tables: HashMap<ObjectID, Vec<SymbolTableEntry>>,
    pub global_symtable: BTreeMap<SymbolName, (Option<Defn>, Refs)>,
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
        let global_symtable = BTreeMap::new();
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

pub enum Endianness {
    BigEndian,
    LittleEndian,
}

pub struct LinkerEditor {
    text_start: i32,
    data_start_boundary: i32,
    bss_start_boundary: i32,
    session_objects: BTreeMap<ObjectID, ObjectIn>,
    logger: Logger,
    _endianness: Endianness,
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
            session_objects: BTreeMap::new(),
            _endianness: Endianness::BigEndian, // always BigEndian now ...
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
        objs_in: BTreeMap<ObjectID, ObjectIn>,
        static_libs: Vec<StaticLib>,
    ) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        let mut out = ObjectOut::new();
        let mut info = LinkerInfo::new();

        // initial pass over input objects
        for (obj_id, obj) in objs_in.into_iter() {
            self.alloc_storage_and_symtables(&obj_id, &obj, &mut out, &mut info)?;
            self.session_objects.insert(obj_id, obj);
        }

        self.logger
            .debug(format!("Object out (initial allocation):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (initial allocation):\n{}", info.ppr()).as_str());

        let mut undef_syms: Vec<SymbolName> = vec![];
        // check if all definitions are in place. if not - check/link libaries
        for (name, (defn, _)) in info.global_symtable.iter() {
            if defn.is_none() {
                undef_syms.push(name.to_string())
            }
        }
        if !undef_syms.is_empty() {
            self.logger
                .info(&format!("Undefined symbols:\n  {undef_syms:?}"));
            self.logger.info("Checking static libs");
            self.static_libs_symbol_lookup(&mut out, &mut info, &mut undef_syms, &static_libs)?;
        }

        // update segment offsets
        let bss_start = self.patch_segment_offsets(&mut out, &mut info);
        self.logger
            .debug(format!("Object out (segment offset patching):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (segment offset patching):\n{}", info.ppr()).as_str());
        // Implement Unix-style common blocks. That is, scan the symbol table for undefined symbols
        // with non-zero values, and add space of appropriate size to the .bss segment.
        self.common_block_allocation(&mut out, &mut info, bss_start);
        println!("{info:?}");
        // Check for undefined symbols
        if info
            .global_symtable
            .values()
            .any(|(defn, _)| defn.is_none())
        {
            return Err(LinkError::UndefinedSymbolError);
        }

        // resolve global symbols offsets
        self.resolve_global_sym_offsets(&mut info);

        // perform relocations
        self.run_relocations(&mut out, &info)?;

        /////////////////////////////////////////////
        self.logger.debug("Linking complete");
        self.logger
            .debug(format!("Object out (final):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (final):\n{}", info.ppr()).as_str());
        Ok((out, info))
    }

    // Allocate storage and build symbol tables for given module object
    fn alloc_storage_and_symtables(
        &mut self,
        obj_id: &ObjectID,
        obj: &ObjectIn,
        out: &mut ObjectOut,
        info: &mut LinkerInfo,
    ) -> Result<(), LinkError> {
        self.logger.debug(&format!(
            " ==> Linking in {}\n{}",
            obj_id,
            obj.ppr(true).as_str()
        ));
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
                        segment.segment_name, seg_offset, segment.segment_len, out_seg.segment_len
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

    // update TEXT start and patch segment addrs in link info
    // then do the same patching in DATA segment - update start and adjust address in info
    // then BSS. We might reuse the returned value (BSS_START) in case we need to allocate
    // later common block
    fn patch_segment_offsets(&mut self, out: &mut ObjectOut, info: &mut LinkerInfo) -> i32 {
        self.patch_text_seg(out, info);
        self.patch_data_seg(out, info);
        self.patch_bss_seg(out, info)
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
    fn resolve_global_sym_offsets(&self, info: &mut LinkerInfo) {
        for (defn, _) in info.global_symtable.values_mut() {
            if let Some((mod_name, ste_i, addr)) = defn {
                let ste: &SymbolTableEntry = &info.symbol_tables.get(mod_name).unwrap()[*ste_i];
                assert!(ste.st_seg > 0);
                let seg_i = ste.st_seg as usize - 1;
                let sym_seg =
                    &self.session_objects.get(mod_name).unwrap().segments[seg_i].segment_name;
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

    fn static_libs_symbol_lookup(
        &mut self,
        out: &mut ObjectOut,
        info: &mut LinkerInfo,
        undef_syms: &mut Vec<SymbolName>,
        static_libs: &[StaticLib],
    ) -> Result<(), LinkError> {
        let mut visited_libs: HashSet<String> = HashSet::new();
        while !undef_syms.is_empty() {
            let undef_sym = undef_syms.pop().unwrap();

            'outer: for lib in static_libs.iter() {
                match lib {
                    StaticLib::DirLib {
                        symbols, objects, ..
                    } => {
                        for (lib_obj_name, lib_obj_syms) in symbols.iter() {
                            if visited_libs.contains(lib_obj_name) {
                                continue;
                            }
                            for lib_obj_sym in lib_obj_syms.iter() {
                                if lib_obj_sym == &undef_sym {
                                    // found symbol definition in this lib
                                    self.logger.debug(&format!(
                                        "Found symbol '{undef_sym}' in {lib_obj_name}"
                                    ));
                                    if let Some(lib_obj) = objects.get(lib_obj_name).cloned() {
                                        self.alloc_storage_and_symtables(
                                            lib_obj_name,
                                            &lib_obj,
                                            out,
                                            info,
                                        )?;
                                        self.session_objects
                                            .insert(lib_obj_name.to_string(), lib_obj.clone());
                                        for ste in lib_obj.symbol_table.iter() {
                                            if !ste.is_defined() {
                                                undef_syms.push(ste.st_name.to_string());
                                            }
                                        }
                                        visited_libs.insert(lib_obj_name.to_string());
                                        self.logger.debug(&format!(
                                            "Remaining undefined symbols: {undef_syms:?}"
                                        ));
                                    }
                                    break 'outer;
                                }
                            }
                        }
                    }
                    StaticLib::FileLib {
                        symbols,
                        objects,
                        libname,
                    } => {
                        for (lib_obj_sym, obj_offset) in symbols.iter() {
                            if lib_obj_sym == &undef_sym {
                                // found symbol definition in this lib file
                                if let Some(lib_obj) = objects.get(*obj_offset) {
                                    let libobj_id = format!("{libname}_mod_{obj_offset}");
                                    self.logger.debug(&format!(
                                        "Found symbol '{undef_sym}' at offset {obj_offset}"
                                    ));
                                    if visited_libs.contains(&libobj_id) {
                                        continue;
                                    }
                                    self.alloc_storage_and_symtables(
                                        &libobj_id, lib_obj, out, info,
                                    )?;
                                    self.session_objects
                                        .insert(libobj_id.to_string(), lib_obj.clone());
                                    for ste in lib_obj.symbol_table.iter() {
                                        if !ste.is_defined() {
                                            undef_syms.push(ste.st_name.to_string());
                                        }
                                    }
                                    visited_libs.insert(libobj_id);
                                    self.logger.debug(&format!(
                                        "Remaining undefined symbols: {undef_syms:?}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn run_relocations(
        &mut self,
        out: &mut ObjectOut,
        info: &LinkerInfo,
    ) -> Result<(), LinkError> {
        for (modname, mod_obj) in self.session_objects.iter() {
            self.logger
                .debug(&format!("Running relocations for {modname:}"));
            // println!("DEBUG: {mod_obj:?}");
            for r in mod_obj.relocations.iter() {
                let reloc_entity = match r.rel_ref {
                    RelRef::SegmentRef(seg_i) => {
                        format!("segment {}", mod_obj.segments[seg_i].segment_name)
                    }
                    RelRef::SymbolRef(sym_i) => {
                        format!("symbol '{}'", mod_obj.symbol_table[sym_i].st_name)
                    }
                };
                self.logger.debug(&format!(
                    "Relocation {} of {reloc_entity} reference at offset 0x{:X} (segment {})",
                    r.rel_type, r.rel_loc, r.rel_seg
                ));
                match r.rel_type {
                    RelType::A4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: A4 with SymbolRef"),
                            RelRef::SegmentRef(seg_i) => {
                                // what segment are we relocating? note that we are relocating reference
                                // to the segment of module the contains that relocation entry
                                let seg_name = mod_obj.segments[seg_i].segment_name.clone();
                                // absolute segment ref target address
                                let mod_seg_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&seg_name)
                                    .unwrap();
                                match mk_addr_4(mod_seg_off as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(saa) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            let reloc_seg_start =
                                                out.segments.get(&r.rel_seg).unwrap().segment_start
                                                    - info
                                                        .segment_mapping
                                                        .get(modname)
                                                        .unwrap()
                                                        .get(&r.rel_seg)
                                                        .unwrap();
                                            let reloc_seg_off = reloc_seg_start + r.rel_loc;
                                            self.logger
                                                .debug(&format!("  Setting 0x{mod_seg_off:08X}"));
                                            sd.update(reloc_seg_off as usize, 4, saa);
                                        })
                                    }
                                };
                            }
                        };
                    }
                    RelType::R4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: R4 with SymbolRef"),
                            RelRef::SegmentRef(seg_i) => {
                                let seg_name = mod_obj.segments[seg_i].segment_name.clone();
                                let mod_seg_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&seg_name)
                                    .unwrap();
                                // relocation loc + 4
                                let next_insr_loc = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    + 4;
                                // fix up the code!
                                out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                    let loc_off = next_insr_loc
                                        - 4
                                        - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                    let addend =
                                        x_to_i4(sd.get_at(loc_off as usize, 0x4).unwrap()).unwrap();
                                    let rel_addr_val = mk_i_4(next_insr_loc - mod_seg_off + addend);
                                    self.logger.debug(&format!(
                                        "  Setting 0x{:08X}",
                                        next_insr_loc - mod_seg_off + addend
                                    ));
                                    sd.update(loc_off as usize, 4, rel_addr_val);
                                });
                            }
                        }
                    }
                    RelType::AS4 => {
                        match r.rel_ref {
                            RelRef::SegmentRef(_) => panic!("run_relocations: AS4 with SegmentRef"),
                            RelRef::SymbolRef(sym_i) => {
                                // what symbol are we relocating? note that we are relocating reference
                                // to the segment of module the contains that relocation entry
                                let sym_name = &mod_obj.symbol_table[sym_i].st_name;
                                // absolute symbol ref target address
                                let mod_sym_off = info
                                    .global_symtable
                                    .get(sym_name)
                                    .unwrap()
                                    .0
                                    .as_ref()
                                    .unwrap()
                                    .2
                                    .unwrap();
                                let loc_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                let addend = x_to_i4(
                                    out.object_data
                                        .get(&r.rel_seg)
                                        .unwrap()
                                        .get_at(loc_off as usize, 0x4)
                                        .unwrap(),
                                )
                                .unwrap();
                                match mk_addr_4((mod_sym_off + addend) as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting 0x{:08X}",
                                                mod_sym_off + addend
                                            ));
                                            sd.update(loc_off as usize, 4, v);
                                        });
                                    }
                                }
                            }
                        }
                    }
                    RelType::RS4 => match r.rel_ref {
                        RelRef::SegmentRef(_) => panic!("run_relocations: RS4 with SegmentRef"),
                        RelRef::SymbolRef(sym_i) => {
                            let sym_name = &mod_obj.symbol_table[sym_i].st_name;
                            // absolute symbol ref target address
                            let mod_sym_off = info
                                .global_symtable
                                .get(sym_name)
                                .unwrap()
                                .0
                                .as_ref()
                                .unwrap()
                                .2
                                .unwrap();
                            let loc_addr = *info
                                .segment_mapping
                                .get(modname)
                                .unwrap()
                                .get(&r.rel_seg)
                                .unwrap();
                            let loc_off = loc_addr + r.rel_loc
                                - out.segments.get(&r.rel_seg).unwrap().segment_start;
                            let addend = x_to_i4(
                                out.object_data
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    .get_at(loc_off as usize, 0x4)
                                    .unwrap(),
                            )
                            .unwrap();
                            // fix up the code!
                            out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                let rel_addr_val = mk_i_4(loc_addr + 4 - mod_sym_off + addend);
                                self.logger.debug(&format!(
                                    "  Setting 0x{:08X}",
                                    loc_addr + 4 - mod_sym_off + addend
                                ));
                                sd.update(loc_off as usize, 0x4, rel_addr_val);
                            });
                        }
                    },
                    RelType::U2 => {
                        match r.rel_ref {
                            RelRef::SegmentRef(_) => panic!("run_relocations: U2 with SegmentRef"),
                            RelRef::SymbolRef(sym_i) => {
                                // what symbol are we relocating? note that we are relocating reference
                                // to the segment of module the contains that relocation entry
                                let sym_name = &mod_obj.symbol_table[sym_i].st_name;
                                // absolute symbol ref target address
                                let mod_sym_off = info
                                    .global_symtable
                                    .get(sym_name)
                                    .unwrap()
                                    .0
                                    .as_ref()
                                    .unwrap()
                                    .2
                                    .unwrap();
                                let loc_addr = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap();
                                let loc_off = loc_addr + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                match mk_addr_4(mod_sym_off as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting 0x{:04X}",
                                                x_to_i2(&v[0..2]).unwrap()
                                            ));
                                            sd.update(loc_off as usize, 2, v[0..2].to_vec());
                                        });
                                    }
                                }
                            }
                        }
                    }
                    RelType::L2 => {
                        match r.rel_ref {
                            RelRef::SegmentRef(_) => panic!("run_relocations: L2 with SegmentRef"),
                            RelRef::SymbolRef(sym_i) => {
                                // what symbol are we relocating? note that we are relocating reference
                                // to the segment of module the contains that relocation entry
                                let sym_name = &mod_obj.symbol_table[sym_i].st_name;
                                // absolute symbol ref target address
                                let mod_sym_off = info
                                    .global_symtable
                                    .get(sym_name)
                                    .unwrap()
                                    .0
                                    .as_ref()
                                    .unwrap()
                                    .2
                                    .unwrap();
                                let loc_addr = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap();
                                let loc_off = loc_addr + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                match mk_addr_4(mod_sym_off as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting 0x{:04X}",
                                                x_to_i2(&v[2..4]).unwrap()
                                            ));
                                            sd.update(loc_off as usize, 2, v[2..4].to_vec());
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
