use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Deref;

// use either::Either::{Left, Right};

use crate::common::{Defn, ObjectID, Refs};
use crate::types::errors::LinkError;
use crate::types::library::StaticLib;
use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;
use crate::types::relocation::{RelRef, RelType, Relocation};
use crate::types::segment::{Segment, SegmentData, SegmentName};
// use crate::types::stub::StubMember;
use crate::types::symbol_table::{SymbolName, SymbolTableEntry};
use crate::utils::{find_seg_start, mk_addr_4, mk_i_4, x_to_i2, x_to_i4};
use crate::{logger::*, wrapped_symbol};

pub enum LinkObjType {
    SharedLib,
    Executable,
}

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
        for (obj_id, seg_addrs) in self.segment_mapping.iter() {
            let mut entry = String::new();
            entry.push_str(format!("  {} =>", &obj_id).as_str());
            for s_n in SegmentName::order().iter() {
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
    text_start: i32, // exe/lib start
    data_start_boundary: i32,
    bss_start_boundary: i32,
    pub session_objects: BTreeMap<ObjectID, ObjectIn>,
    logger: Logger,
    _endianness: Endianness,
}

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

    pub fn link(
        &mut self,
        objs_in: BTreeMap<ObjectID, ObjectIn>,
        static_libs: Vec<StaticLib>,
        wrap_routines: Vec<SymbolName>,
    ) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        self.do_link(objs_in, static_libs, wrap_routines, LinkObjType::Executable)
    }

    pub fn link_lib(
        &mut self,
        objs_in: BTreeMap<ObjectID, ObjectIn>,
        static_libs: Vec<StaticLib>,
        wrap_routines: Vec<SymbolName>,
    ) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        self.do_link(objs_in, static_libs, wrap_routines, LinkObjType::SharedLib)
    }

    // for each object_in
    // for each segment in object_in
    //   * allocate storage in object_out
    //   * update address mappings in link_info
    //   * resolve symbol addresses
    //   * do the relocation fixups
    fn do_link(
        &mut self,
        mut objs_in: BTreeMap<ObjectID, ObjectIn>,
        static_libs: Vec<StaticLib>,
        wrap_routines: Vec<SymbolName>,
        _link_obj_ty: LinkObjType,
    ) -> Result<(ObjectOut, LinkerInfo), LinkError> {
        let mut out = ObjectOut::new();
        let mut info = LinkerInfo::new();

        // wrap specified routines
        self.wrap_routines(&mut objs_in, &wrap_routines)?;

        // initial pass over input objects
        let mut got_size = 0;
        for (obj_id, obj) in objs_in.into_iter() {
            got_size += self.alloc_storage_and_symtables(&obj_id, &obj, &mut out, &mut info)?;
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
                undef_syms.push(name.clone());
            }
        }
        if !undef_syms.is_empty() {
            self.logger
                .info(&format!("Undefined symbols:\n  {undef_syms:?}"));
            self.logger.info("Checking static libs");
            self.static_libs_symbol_lookup(&mut out, &mut info, &mut undef_syms, &static_libs)?;
        }

        // update segment offsets
        let bss_start = self.patch_segment_offsets(&mut out, &mut info, got_size);
        self.logger
            .debug(format!("Object out (segment offset patching):\n{}", out.ppr()).as_str());
        self.logger
            .debug(format!("Info (segment offset patching):\n{}", info.ppr()).as_str());

        // Implement Unix-style common blocks. That is, scan the symbol table for undefined symbols
        // with non-zero values, and add space of appropriate size to the .bss segment.
        self.common_block_allocation(&mut out, &mut info, bss_start);

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
    ) -> Result<i32, LinkError> {
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

        let mut got_size = 0;
        for r in obj.relocations.iter() {
            if r.rel_type == RelType::GP4 {
                got_size += 4;
            }
        }

        Ok(got_size)
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
                .entry(symbol.st_name.clone())
                .and_modify(|(defn, refs)| {
                    if symbol.is_defined() {
                        assert!(defn.is_none());
                        *defn = Some(Defn::new(obj_id.to_string(), i, None));
                    } else {
                        refs.insert(obj_id.to_string(), i);
                    }
                })
                .or_insert_with(|| {
                    if symbol.is_defined() {
                        (Some(Defn::new(obj_id.to_string(), i, None)), HashMap::new())
                    } else {
                        let mut refs = HashMap::new();
                        refs.insert(obj_id.to_string(), i);
                        (None, refs)
                    }
                });
        }
        None
    }

    // Update TEXT start and patch segment addrs in link info
    // If we are building PiC - factor in and allocate global offset table.
    // Then do the same patching in DATA segment - update start and adjust address in info.
    // Then BSS. We might reuse the returned value (BSS_START) in case we need to allocate
    // common block later.
    fn patch_segment_offsets(
        &mut self,
        out: &mut ObjectOut,
        info: &mut LinkerInfo,
        got_size: i32,
    ) -> i32 {
        self.patch_text_seg(out, info);
        if got_size != 0 {
            self.logger.debug("GOT segment will be allocated");
            self.alloc_got(out, got_size);
        }
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

    fn alloc_got(&self, out: &mut ObjectOut, got_size: i32) {
        let mut got_segment = Segment::new(SegmentName::GOT);
        let text_end = out.segments.get(&SegmentName::TEXT).unwrap().segment_start
            + out.segments.get(&SegmentName::TEXT).unwrap().segment_len;
        got_segment.segment_start = text_end;
        got_segment.segment_len = got_size;
        out.segments.insert(SegmentName::GOT, got_segment);
        out.object_data
            .insert(SegmentName::GOT, SegmentData::new(got_size as usize));
    }

    fn patch_data_seg(&mut self, out: &mut ObjectOut, info: &mut LinkerInfo) {
        let last_seg_name = match out.segments.get(&SegmentName::GOT) {
            Some(_) => SegmentName::GOT,
            None => SegmentName::TEXT,
        };
        let last_seg_end = out.segments.get(&last_seg_name).unwrap().segment_start
            + out.segments.get(&last_seg_name).unwrap().segment_len;
        let data_start = find_seg_start(last_seg_end, self.data_start_boundary);
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
            if let Some(Defn {
                defn_mod_id,
                defn_ste_ix: Some(ste_ix),
                defn_addr,
                ..
            }) = defn
            {
                let ste: &SymbolTableEntry = &info.symbol_tables.get(defn_mod_id).unwrap()[*ste_ix];
                assert!(ste.st_seg > 0);
                let seg_i = ste.st_seg as usize - 1;
                let sym_seg =
                    &self.session_objects.get(defn_mod_id).unwrap().segments[seg_i].segment_name;
                let segment_offset = *info
                    .segment_mapping
                    .get(defn_mod_id)
                    .unwrap()
                    .get(sym_seg)
                    .unwrap();
                *defn_addr = Some(segment_offset + ste.st_value);
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
        let mut visited_libs_objs: HashSet<String> = HashSet::new();
        while !undef_syms.is_empty() {
            let undef_sym = undef_syms.pop().unwrap();

            'outer: for lib in static_libs.iter() {
                match lib {
                    StaticLib::DirLib {
                        symbols, objects, ..
                    } => {
                        for (lib_obj_name, lib_obj_syms) in symbols.iter() {
                            if visited_libs_objs.contains(lib_obj_name) {
                                continue;
                            }
                            for lib_obj_sym in lib_obj_syms.iter() {
                                if *lib_obj_sym == undef_sym {
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
                                                undef_syms.push(ste.st_name.clone());
                                            }
                                        }
                                        visited_libs_objs.insert(lib_obj_name.to_string());
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
                            if *lib_obj_sym == undef_sym {
                                // found symbol definition in this lib file
                                if let Some(lib_obj) = objects.get(*obj_offset) {
                                    let libobj_id = format!("{libname}_mod_{obj_offset}");
                                    self.logger.debug(&format!(
                                        "Found symbol '{undef_sym}' at offset {obj_offset}"
                                    ));
                                    if visited_libs_objs.contains(&libobj_id) {
                                        continue;
                                    }
                                    self.alloc_storage_and_symtables(
                                        &libobj_id, lib_obj, out, info,
                                    )?;
                                    self.session_objects
                                        .insert(libobj_id.to_string(), lib_obj.clone());
                                    for ste in lib_obj.symbol_table.iter() {
                                        if !ste.is_defined() {
                                            undef_syms.push(ste.st_name.clone());
                                        }
                                    }
                                    visited_libs_objs.insert(libobj_id);
                                    self.logger.debug(&format!(
                                        "Remaining undefined symbols: {undef_syms:?}"
                                    ));
                                }
                            }
                        }
                    }
                    StaticLib::Stub(stublib) => {
                        for (membername, stub) in stublib.members.iter() {
                            let libobj_id = format!("{}_{membername}", stublib.libname);
                            if visited_libs_objs.contains(&libobj_id) {
                                continue;
                            }
                            if stub.syms.contains_key(&undef_sym) {
                                // self.add_shared_lib_defn(info, stub, &undef_sym);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn run_relocations(&mut self, out: &mut ObjectOut, info: &LinkerInfo) -> Result<(), LinkError> {
        let mut got_offset = 0;
        for (modname, mod_obj) in self.session_objects.iter() {
            if !mod_obj.relocations.is_empty() {
                self.logger
                    .debug(&format!("Running relocations for {modname:}"));
            }
            // println!("DEBUG: {mod_obj:?}");
            for r in mod_obj.relocations.iter() {
                let reloc_entity = match r.rel_ref {
                    RelRef::SegmentRef(seg_i) => {
                        format!("segment {} reference", mod_obj.segments[seg_i].segment_name)
                    }
                    RelRef::SymbolRef(sym_i) => {
                        format!("symbol '{}' reference", mod_obj.symbol_table[sym_i].st_name)
                    }
                    RelRef::NoRef => String::new(),
                };
                self.logger.debug(&format!(
                    "Relocation {} of {reloc_entity} at offset 0x{:X} (segment {})",
                    r.rel_type, r.rel_loc, r.rel_seg
                ));
                match r.rel_type {
                    RelType::A4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: A4 with SymbolRef"),
                            RelRef::NoRef => panic!("run_relocations: A4 with NoRef"),
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
                                // create PiC relocations
                                let er_rel_loc = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                self.logger.debug(&format!(
                                    "  Creating ER4 relocation at 0x{er_rel_loc:08X}"
                                ));
                                out.relocations.push(Relocation {
                                    rel_loc: er_rel_loc,
                                    rel_seg: r.rel_seg.clone(),
                                    rel_ref: RelRef::NoRef,
                                    rel_type: RelType::ER4,
                                });
                            }
                        };
                    }
                    RelType::R4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: R4 with SymbolRef"),
                            RelRef::NoRef => panic!("run_relocations: R4 with NoRef"),
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
                            RelRef::NoRef => panic!("run_relocations: AS4 with NoRef"),
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
                                    .defn_addr
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
                                // create PiC relocations
                                let er_rel_loc = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                self.logger.debug(&format!(
                                    "  Creating ER4 relocation at 0x{er_rel_loc:08X}"
                                ));
                                out.relocations.push(Relocation {
                                    rel_loc: er_rel_loc,
                                    rel_seg: r.rel_seg.clone(),
                                    rel_ref: RelRef::NoRef,
                                    rel_type: RelType::ER4,
                                });
                            }
                        }
                    }
                    RelType::RS4 => match r.rel_ref {
                        RelRef::SegmentRef(_) => panic!("run_relocations: RS4 with SegmentRef"),
                        RelRef::NoRef => panic!("run_relocations: RS4 with NoRef"),
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
                                .defn_addr
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
                            RelRef::NoRef => panic!("run_relocations: U2 with NoRef"),
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
                                    .defn_addr
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
                            RelRef::NoRef => panic!("run_relocations: L2 with NoRef"),
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
                                    .defn_addr
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
                    RelType::GA4 => {
                        match r.rel_ref {
                            RelRef::SegmentRef(_) => panic!("run_relocations: GA4 with SegmentRef"),
                            RelRef::SymbolRef(_) => panic!("run_relocations: GA4 with SymbolRef"),
                            RelRef::NoRef => {
                                let seg_addr = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap();
                                let loc_off = seg_addr + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                let got_off =
                                    out.segments.get(&SegmentName::GOT).unwrap().segment_start;
                                let dist_to_got = got_off - (seg_addr + r.rel_loc);
                                match mk_addr_4(dist_to_got as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger
                                                .debug(&format!("  Setting 0x{dist_to_got:08X}",));
                                            sd.update(loc_off as usize, 4, v[0..4].to_vec());
                                        });
                                    }
                                }
                            }
                        }
                    }
                    RelType::GP4 => {
                        match r.rel_ref {
                            RelRef::SegmentRef(_) => panic!("run_relocations: GP4 with SegmentRef"),
                            RelRef::NoRef => panic!("run_relocations: GP4 with NoRef"),
                            RelRef::SymbolRef(sym_i) => {
                                let sz = 4;
                                let sym_name = &mod_obj.symbol_table[sym_i].st_name;
                                let mod_sym_off = info
                                    .global_symtable
                                    .get(sym_name)
                                    .unwrap()
                                    .0
                                    .as_ref()
                                    .unwrap()
                                    .defn_addr
                                    .unwrap();
                                match mk_addr_4((mod_sym_off) as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(SegmentName::GOT).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting 0x{mod_sym_off:08X} in GOT at offset {got_offset}"
                                            ));
                                            sd.update(got_offset, sz, v);
                                        });
                                    }
                                }
                                let loc_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                match mk_addr_4(got_offset) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting GOT offset 0x{got_offset:08X} in {}",
                                                r.rel_seg
                                            ));
                                            sd.update(loc_off as usize, sz, v);
                                        });
                                    }
                                }
                                got_offset += sz;
                            }
                        }
                    }
                    RelType::GR4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: GR4 with SymbolRef"),
                            RelRef::NoRef => panic!("run_relocations: GR4 with NoRef"),
                            RelRef::SegmentRef(seg_i) => {
                                let loc_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                let addr_off = x_to_i4(
                                    out.object_data
                                        .get(&r.rel_seg)
                                        .unwrap()
                                        .get_at(loc_off as usize, 0x4)
                                        .unwrap(),
                                )
                                .unwrap();
                                let seg_name = mod_obj.segments[seg_i].segment_name.clone();
                                let seg_ref_addr = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&seg_name)
                                    .unwrap();
                                let got_off =
                                    out.segments.get(&SegmentName::GOT).unwrap().segment_start;
                                // fix up the code!
                                out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                    let rel_addr_val = mk_i_4(seg_ref_addr + addr_off - got_off);
                                    self.logger.debug(&format!(
                                        "  Setting 0x{:08X}",
                                        seg_ref_addr + addr_off - got_off
                                    ));
                                    sd.update(loc_off as usize, 4, rel_addr_val);
                                });
                            }
                        }
                    }
                    RelType::ER4 => {
                        match r.rel_ref {
                            RelRef::SymbolRef(_) => panic!("run_relocations: ER4 with SymbolRef"),
                            RelRef::SegmentRef(_) => panic!("run_relocations: ER4 with SegmentRef"),
                            RelRef::NoRef => {
                                let loc_off = *info
                                    .segment_mapping
                                    .get(modname)
                                    .unwrap()
                                    .get(&r.rel_seg)
                                    .unwrap()
                                    + r.rel_loc
                                    - out.segments.get(&r.rel_seg).unwrap().segment_start;
                                let addr = x_to_i4(
                                    out.object_data
                                        .get(&r.rel_seg)
                                        .unwrap()
                                        .get_at(loc_off as usize, 0x4)
                                        .unwrap(),
                                )
                                .unwrap();
                                match mk_addr_4((addr + self.text_start) as usize) {
                                    None => return Err(LinkError::AddressOverflowError),
                                    Some(v) => {
                                        // fix up the code!
                                        out.object_data.entry(r.rel_seg.clone()).and_modify(|sd| {
                                            self.logger.debug(&format!(
                                                "  Setting 0x{:08X}",
                                                addr + self.text_start
                                            ));
                                            sd.update(loc_off as usize, 4, v);
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

    fn wrap_routines(
        &mut self,
        objs_in: &mut BTreeMap<ObjectID, ObjectIn>,
        routine_names: &[SymbolName],
    ) -> Result<(), LinkError> {
        let mut already_wrapped = HashSet::new();
        for (_, obj) in objs_in.iter_mut() {
            for sym in obj.symbol_table.iter_mut() {
                if sym.st_name.deref().starts_with("wrap_")
                    || sym.st_name.deref().starts_with("real_")
                {
                    let n = sym.st_name.deref()[5..].to_owned();
                    if already_wrapped.contains(&wrapped_symbol!(n)) {
                        return Err(LinkError::WrappedSymbolNameAlreadyExists);
                    }
                }
                if routine_names.contains(&sym.st_name) {
                    sym.st_name = SymbolName::WrappedSName(sym.st_name.deref().to_owned());
                    already_wrapped.insert(&sym.st_name);
                }
            }
        }
        Ok(())
    }

    // // this assumes reference is indeed defined in given stub member
    // fn add_shared_lib_defn(&self, info: &mut LinkerInfo, stub0: &StubMember, sym: &SymbolName) -> Result<(), LinkError> {
    //     assert!(stub0.syms.contains_key(sym));
    //     let visited_members: HashSet<&str> = HashSet::new();
    //     let stub_libs = vec![stub0];
    //     while let Some(stub) = stub_libs.pop() {
    //         // if visited_members.contains(&stub.name) {
    //         //     return Err(LinkError::SharedLibsReferenceCycle);
    //         // }
    //         match stub.syms.get(sym) {
    //             None => {
    //                 return Err(LinkError::SharedLibRefDefnNotFound)
    //             },
    //             Some(Right(libname)) => {
    //                 self.logger.debug(&format!(" Found defn for symbol '{sym}' in {}\n", stub.name));

    //             },
    //             Some(Left(addr)) => {
    //                 self.logger.debug(&format!(" Found defn for symbol '{sym}' in {}\n", stub.name));
    //                 info.global_symtable
    //                     .entry(sym.to_owned())
    //                     .and_modify(|(defn, _refs)| {
    //                         assert!(defn.is_none());
    //                         *defn = Some(Defn::shared_lib_defn(stub.name, *addr));
    //                     });
    //                 break;
    //             }
    //         }
    //     }
    //     Ok(())
    // }
}
