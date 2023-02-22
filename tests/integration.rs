use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
// use linkerloader::gen::gen_obj_data;
use linkerloader::lib::{parse_object, read_lib, read_objects, read_objects_from_dir};
use linkerloader::librarian::Librarian;
use linkerloader::linker::editor::LinkerEditor;
use linkerloader::types::errors::{LinkError, ParseError};
use linkerloader::types::library::StaticLib;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::types::relocation::{RelRef, RelType, Relocation};
use linkerloader::types::segment::{SegmentDescr, SegmentName};
use linkerloader::types::symbol_table::{SymbolName, SymbolTableEntry, SymbolTableEntryType};
use linkerloader::utils::{read_object_file, x_to_i2, x_to_i4};
use linkerloader::{symbol, wrapped_symbol};

const TESTS_DIR: &'static str = "tests/input/";
const NO_STATIC_LIBS: Vec<StaticLib> = vec![];
const NO_WRAP_ROUTINES: Vec<SymbolName> = vec![];

fn ensure_clean_state(path: &str) {
    ensure_clean_state_extra(path, vec![]);
}

fn ensure_clean_state_extra(path: &str, other_things_to_clean: Vec<&str>) {
    println!("test cleanup");
    let p = &PathBuf::from(path);
    if p.exists() {
        // delete static libdir  if exists
        let static_lib = p.join(PathBuf::from("staticlib"));
        if static_lib.exists() {
            println!("removing static lib dir");
            fs::remove_dir_all(static_lib).unwrap();
        }
        // delete static lib file if exists
        let static_lib = p.join(PathBuf::from("staticlibfile"));
        if static_lib.exists() {
            println!("removing static lib file");
            fs::remove_file(static_lib).unwrap();
        }
    }
    for s in other_things_to_clean.into_iter() {
        let p1 = &p.join(PathBuf::from(s));
        if p1.exists() {
            if p1.is_dir() {
                let _ = fs::remove_dir_all(p1);
            }
            if p1.is_file() {
                let _ = fs::remove_file(p1);
            }
        }
    }
}

#[test]
fn test_magic_number_simple() {
    let obj_file = read_object_file(&tests_base_loc("simple"));
    let magic_number = obj_file.lines().next().unwrap();
    assert_eq!(MAGIC_NUMBER, magic_number);
}

fn test_failure(e0: ParseError, fp: &str) {
    let res = parse_object(fp);
    if res.is_ok() {
        println!("{:?}", res);
        assert!(res.is_err());
    }
    match res {
        Ok(_) => {
            panic!("unexpected");
        }
        Err(e) => assert_eq!(e0, e),
    }
}

fn multi_object_test(dirname: &str) {
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x100, 0x100, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, _info)) => {
            assert_eq!(out.nsegs as usize, out.segments.len());
            assert_eq!(out.object_data.len(), out.segments.len());
            let text_seg = out
                .segments
                .get(&SegmentName::TEXT)
                .unwrap_or_else(|| panic!("failed to get text segment"));
            let text_seg_data = out
                .object_data
                .get(&SegmentName::TEXT)
                .unwrap_or_else(|| panic!("failed to get text code / data"));
            assert_eq!(text_seg.segment_len as usize, text_seg_data.len());
            let data_seg = out
                .segments
                .get(&SegmentName::DATA)
                .unwrap_or_else(|| panic!("failed to get data segment"));
            let data_seg_data = out
                .object_data
                .get(&SegmentName::DATA)
                .unwrap_or_else(|| panic!("failed to get data code / data"));
            assert_eq!(data_seg.segment_len as usize, data_seg_data.len());
            let bss_seg = out
                .segments
                .get(&SegmentName::BSS)
                .unwrap_or_else(|| panic!("failed to get bss segment"));
            let bss_seg_data = out
                .object_data
                .get(&SegmentName::BSS)
                .unwrap_or_else(|| panic!("failed to get bss code / data"));
            assert_eq!(bss_seg.segment_len as usize, bss_seg_data.len());
        }
        Err(_e) => panic!("{}", dirname),
    }
}

fn tests_base_loc(name: &str) -> String {
    format!("{}{}", TESTS_DIR, name)
}

#[test]
fn magic_number_not_present() {
    test_failure(
        ParseError::MissingMagicNumber,
        &tests_base_loc("no_magic_number"),
    );
}

#[test]
fn invalid_magic_number() {
    test_failure(
        ParseError::InvalidMagicNumber,
        &tests_base_loc("invalid_magic_number"),
    );
}

#[test]
fn missing_nsegs_nsums_nrels() {
    test_failure(
        ParseError::MissingNSegsNSumsNRels,
        &tests_base_loc("missing_nsegs_nsums_nrels"),
    );
}

#[test]
fn invalid_nsegs_nsums_nrels() {
    test_failure(
        ParseError::InvalidNSegsNSumsNRels,
        &tests_base_loc("invalid_nsegs_nsums_nrels"),
    );
}

#[test]
fn invalid_nsegs() {
    test_failure(
        ParseError::InvalidNSegsValue,
        &tests_base_loc("invalid_nsegs"),
    );
}

#[test]
fn invalid_nsyms() {
    test_failure(
        ParseError::InvalidNSymsValue,
        &tests_base_loc("invalid_nsyms"),
    );
}

#[test]
fn invalid_nrels() {
    test_failure(
        ParseError::InvalidNRelsValue,
        &tests_base_loc("invalid_nrels"),
    );
}

#[test]
fn invalid_segment_name() {
    test_failure(
        ParseError::InvalidSegmentName,
        &tests_base_loc("invalid_segment_name"),
    );
}

#[test]
fn invalid_segment_start() {
    test_failure(
        ParseError::InvalidSegmentStart,
        &tests_base_loc("invalid_segment_start"),
    );
}

#[test]
fn invalid_segment_len() {
    test_failure(
        ParseError::InvalidSegmentLen,
        &tests_base_loc("invalid_segment_len"),
    );
}

#[test]
fn invalid_segment_descr() {
    test_failure(
        ParseError::InvalidSegmentDescr,
        &tests_base_loc("invalid_segment_descr"),
    );
}

#[test]
fn invalid_num_of_segs_1() {
    test_failure(
        ParseError::InvalidNumOfSegments,
        &tests_base_loc("invalid_num_of_segs_1"),
    );
}

#[test]
fn invalid_num_of_segs_2() {
    test_failure(
        ParseError::InvalidNumOfSegments,
        &tests_base_loc("invalid_num_of_segs_2"),
    );
}

#[test]
fn invalid_num_of_segs_3() {
    test_failure(
        ParseError::InvalidNumOfSegments,
        &tests_base_loc("invalid_num_of_segs_3"),
    );
}

#[test]
fn invalid_num_of_segs_4() {
    test_failure(
        ParseError::InvalidNumOfSegments,
        &tests_base_loc("invalid_num_of_segs_4"),
    );
}

#[test]
fn segments() {
    let res = parse_object(&tests_base_loc("segments_1"));
    println!("{:?}", res);
    assert!(res.is_ok());
    match res {
        Err(_) => panic!("unexpected"),
        Ok(obj) => {
            assert_eq!(obj.nsegs, obj.segments.len() as i32);
            let seg1 = &obj.segments[0];
            assert_eq!(SegmentName::TEXT, seg1.segment_name);
            assert_eq!(0x0, seg1.segment_start);
            assert_eq!(0x32, seg1.segment_len);
            assert_eq!(SegmentDescr::R, seg1.segment_descr[0]);
            assert_eq!(SegmentDescr::P, seg1.segment_descr[1]);
            assert_eq!(0x32, obj.object_data[0].deref().len());
            assert_eq!(0x46, obj.object_data[2].deref().len());
        }
    }
}

#[test]
fn invalid_symbol_table_entry() {
    test_failure(
        ParseError::InvalidSymbolTableEntry,
        &tests_base_loc("invalid_symbol_table_entry"),
    );
}

#[test]
fn invalid_symbol_table_entry_seg() {
    test_failure(
        ParseError::InvalidSTESegment,
        &tests_base_loc("invalid_symbol_table_entry_seg"),
    );
}

#[test]
fn invalid_symbol_table_type() {
    test_failure(
        ParseError::InvalidSTEType,
        &tests_base_loc("invalid_symbol_table_entry_type"),
    );
}

#[test]
fn invalid_symbol_table_value() {
    test_failure(
        ParseError::InvalidSTEValue,
        &tests_base_loc("invalid_symbol_table_entry_value"),
    );
}

#[test]
fn invalid_symbol_table_segment_out_of_range() {
    test_failure(
        ParseError::STESegmentRefOutOfRange,
        &tests_base_loc("invalid_symbol_table_seg_out_of_range"),
    );
}

#[test]
fn symbol_table() {
    let res = parse_object(&tests_base_loc("symbol_table_1"));
    println!("{:?}", res);
    assert!(res.is_ok());
    match res {
        Err(_) => panic!("unexpected"),
        Ok(obj) => {
            assert_eq!(obj.nsyms, obj.symbol_table.len() as i32);
            let ste1: &SymbolTableEntry = &obj.symbol_table[0];
            assert_eq!("foo", ste1.st_name.deref());
            assert_eq!(0x1a, ste1.st_value);
            assert_eq!(1, ste1.st_seg); // 2500 decimal
            assert_eq!(SymbolTableEntryType::D, ste1.st_type);
            let ste2: &SymbolTableEntry = &obj.symbol_table[1];
            assert_eq!("bas", ste2.st_name.deref());
            assert_eq!(0x2b, ste2.st_value);
            assert_eq!(0, ste2.st_seg); // 2500 decimal
            assert_eq!(SymbolTableEntryType::U, ste2.st_type);
            assert_eq!(0x40, obj.object_data[0].deref().len());
        }
    }
}

#[test]
fn invalid_relocation_entry() {
    test_failure(
        ParseError::InvalidRelocationEntry,
        &tests_base_loc("invalid_reloc_entry"),
    );
}

#[test]
fn invalid_relocation_addr() {
    test_failure(
        ParseError::InvalidRelRef,
        &tests_base_loc("invalid_reloc_addr"),
    );
}

#[test]
fn rel_segment_out_of_range() {
    test_failure(
        ParseError::RelSegmentOutOfRange,
        &tests_base_loc("reloc_segment_out_of_range"),
    );
}

#[test]
fn rel_symbol_out_of_range() {
    test_failure(
        ParseError::RelSymbolOutOfRange,
        &tests_base_loc("reloc_symbol_out_of_range"),
    );
}

#[test]
fn invalid_reloc_type() {
    test_failure(
        ParseError::InvalidRelType,
        &tests_base_loc("invalid_reloc_type"),
    );
}

#[test]
fn invalid_reloc_segment() {
    test_failure(
        ParseError::InvalidRelSegment,
        &tests_base_loc("invalid_reloc_segment"),
    );
}

#[test]
fn invalid_num_of_relocations_1() {
    test_failure(
        ParseError::InvalidNumOfRelocations,
        &tests_base_loc("invalid_num_of_relocations_1"),
    );
}

#[test]
fn invalid_num_of_relocations_2() {
    test_failure(
        ParseError::InvalidNumOfRelocations,
        &tests_base_loc("invalid_num_of_relocations_2"),
    );
}

#[test]
fn relocations() {
    let res = parse_object(&tests_base_loc("relocations_1"));
    println!("{:?}", res);
    assert!(res.is_ok());
    match res {
        Err(_) => panic!("unexpected"),
        Ok(obj) => {
            assert_eq!(obj.nrels, obj.relocations.len() as i32);
            let rel1: &Relocation = &obj.relocations[0];
            assert_eq!(0x14, rel1.rel_loc);
            assert_eq!(SegmentName::TEXT, rel1.rel_seg);
            assert_eq!(RelRef::SymbolRef(0), rel1.rel_ref);
            assert_eq!(RelType::RS4, rel1.rel_type);
            let rel2: &Relocation = &obj.relocations[1];
            assert_eq!(0x1a, rel2.rel_loc);
            assert_eq!(SegmentName::TEXT, rel2.rel_seg);
            assert_eq!(RelRef::SymbolRef(1), rel2.rel_ref);
            assert_eq!(RelType::RS4, rel2.rel_type);
            assert_eq!(0x33, obj.object_data[0].deref().len());
        }
    }
}

#[test]
fn invalid_object_data() {
    test_failure(
        ParseError::InvalidObjectData,
        &tests_base_loc("invalid_object_data"),
    );
}

#[test]
fn segment_data_len_mismatch() {
    test_failure(
        ParseError::SegmentDataLengthMismatch,
        &tests_base_loc("segment_data_len_mismatch"),
    );
}

#[test]
fn segment_data_out_of_bounds() {
    test_failure(
        ParseError::SegmentDataOutOfBounds,
        &tests_base_loc("segment_data_out_of_bounds"),
    );
}

#[test]
fn link_1() {
    multi_object_test("link_1");
}

#[test]
fn link_2() {
    multi_object_test("link_2");
}

#[test]
fn common_block_1() {
    let dirname = "common_block_1";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            assert_eq!(3, info.common_block_mapping.len());
            assert_eq!(out.object_data.len(), out.segments.len());
            let bss_seg = out
                .segments
                .get(&SegmentName::BSS)
                .unwrap_or_else(|| panic!("failed to get bss segment"));
            let bss_seg_data = out
                .object_data
                .get(&SegmentName::BSS)
                .unwrap_or_else(|| panic!("failed to get bss code / data"));
            let common_block: i32 = info.common_block_mapping.values().sum();
            assert_eq!(
                bss_seg.segment_len as usize,
                bss_seg_data.len() + common_block as usize
            );
        }
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn common_block_bigger_size() {
    let dirname = "common_block_bigger_size";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            assert_eq!(1, info.common_block_mapping.len());
            assert_eq!(out.nsegs as usize, out.segments.len());
            let bss_seg = out
                .segments
                .get(&SegmentName::BSS)
                .unwrap_or_else(|| panic!("failed to get bss segment"));
            let common_block: i32 = info.common_block_mapping.values().sum();
            assert_eq!(bss_seg.segment_len as usize, 0xA);
            assert_eq!(common_block, 0xA);
        }
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn symbol_name_resolution_1() {
    let dirname = "symbol_name_resolution_1";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((_out, info)) => {
            assert_eq!(2, info.global_symtable.len());
            assert!(info.global_symtable.contains_key(&symbol!("foo")));
            assert!(info.global_symtable.contains_key(&symbol!("bar")));
            let foo_ste = info.global_symtable.get(&symbol!("foo")).unwrap().clone();
            assert_eq!("mod_2", foo_ste.0.as_ref().unwrap().0);
            assert_eq!(0, foo_ste.0.as_ref().unwrap().1);
            assert!(foo_ste.1.contains_key("mod_1"));
            assert_eq!(0, *foo_ste.1.get("mod_1").unwrap());
            let bar_ste = info.global_symtable.get(&symbol!("bar")).unwrap().clone();
            assert_eq!("mod_1", bar_ste.0.as_ref().unwrap().0);
            assert_eq!(1, bar_ste.0.as_ref().unwrap().1);
            assert!(bar_ste.1.contains_key("mod_2"));
            assert_eq!(1, *bar_ste.1.get("mod_2").unwrap());
        }
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn multiple_symbol_defns() {
    let dirname = "multiple_symbol_defns";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Err(e) => assert_eq!(LinkError::MultipleSymbolDefinitions, e),
        _ => panic!("{}", dirname),
    }
}

#[test]
fn undefined_symbol() {
    let dirname = "undefined_symbol";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Err(e) => assert_eq!(LinkError::UndefinedSymbolError, e),
        _ => panic!("{}", dirname),
    }
}

#[test]
fn symbol_value_resolution() {
    let dirname = "symbol_value_resolution";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let text_start = 0x10;
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((_out, info)) => {
            println!("{:?}", info);
            assert_eq!(3, info.global_symtable.len());
            let foo_abs_addr = info
                .global_symtable
                .get(&symbol!("foo"))
                .unwrap()
                .0
                .clone()
                .unwrap()
                .2
                .unwrap();
            assert_eq!(0x20, foo_abs_addr);
            let bar_abs_addr = info
                .global_symtable
                .get(&symbol!("bar"))
                .unwrap()
                .0
                .clone()
                .unwrap()
                .2
                .unwrap();
            assert_eq!(0x5A + 0x5, bar_abs_addr);
            let baz_abs_addr = info
                .global_symtable
                .get(&symbol!("baz"))
                .unwrap()
                .0
                .clone()
                .unwrap()
                .2
                .unwrap();
            assert_eq!(0x78 + 0x2, baz_abs_addr);
        }
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn static_lib_dir() {
    let dirname = "static_lib_dir";
    match read_lib(&tests_base_loc(dirname)) {
        Ok(StaticLib::DirLib { symbols, .. }) => {
            assert_eq!(3, symbols.len());
            assert!(symbols.contains_key("libmod_1"));
            assert!(symbols.get("libmod_1").unwrap().contains(&symbol!("foo")));
            assert!(symbols
                .get("libmod_1")
                .unwrap()
                .contains(&symbol!("another_foo")));
            assert!(symbols.contains_key("libmod_2"));
            assert!(symbols.get("libmod_2").unwrap().contains(&symbol!("bar")));
            assert!(symbols.contains_key("libmod_3"));
            assert!(symbols.get("libmod_3").unwrap().contains(&symbol!("baz")));
        }
        Ok(StaticLib::FileLib { .. }) => panic!("unexpected StaticLib::FileLib"),
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn static_lib_file() {
    let dirname = "static_lib_file";
    match read_lib(&tests_base_loc(dirname)) {
        Ok(StaticLib::FileLib { symbols, .. }) => {
            assert_eq!(4, symbols.len());
            assert_eq!(0, *symbols.get(&symbol!("foo")).unwrap());
            assert_eq!(0, *symbols.get(&symbol!("another_foo")).unwrap());
            assert_eq!(1, *symbols.get(&symbol!("bar")).unwrap());
            assert_eq!(2, *symbols.get(&symbol!("baz")).unwrap());
        }
        Ok(StaticLib::DirLib { .. }) => panic!("unexpected StaticLib::DirLib"),
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn build_static_lib_dir() {
    let base_loc = tests_base_loc("build_static_lib_dir");
    ensure_clean_state(&base_loc);
    let objs = vec!["libmod_1", "libmod_2", "libmod_3"];
    let mut librarian = Librarian::new(false);
    match librarian.build_libdir(Some(&base_loc), None, objs) {
        Err(_) => panic!("build_static_lib_dir"),
        Ok(_) => {
            let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib"));
            assert!(lib_loc.exists());
            match read_lib(lib_loc.to_str().unwrap()) {
                Ok(StaticLib::DirLib { symbols, .. }) => {
                    assert_eq!(3, symbols.len());
                    assert!(symbols.contains_key("libmod_1"));
                    assert!(symbols.get("libmod_1").unwrap().contains(&symbol!("foo")));
                    assert!(symbols
                        .get("libmod_1")
                        .unwrap()
                        .contains(&symbol!("another_foo")));
                    assert!(symbols.contains_key("libmod_2"));
                    assert!(symbols.get("libmod_2").unwrap().contains(&symbol!("bar")));
                    assert!(symbols.contains_key("libmod_3"));
                    assert!(symbols.get("libmod_3").unwrap().contains(&symbol!("baz")));
                }
                Ok(StaticLib::FileLib { .. }) => panic!("unexpected StaticLib::FileLib"),
                Err(e) => panic!("build_static_lib_dir: {e:?}"),
            }
        }
    }
    ensure_clean_state(&base_loc);
}

#[test]
fn build_static_lib_file() {
    let base_loc = tests_base_loc("build_static_lib_file");
    ensure_clean_state(&base_loc);
    let objs = vec!["libmod_1", "libmod_2", "libmod_3"];
    let mut librarian = Librarian::new(false);
    match librarian.build_libfile(Some(&base_loc), None, objs) {
        Err(_) => panic!("build_static_lib_file"),
        Ok(_) => {
            let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlibfile"));
            assert!(lib_loc.exists());
            match read_lib(lib_loc.to_str().unwrap()) {
                Ok(StaticLib::FileLib { symbols, .. }) => {
                    assert_eq!(4, symbols.len());
                    assert!(symbols.contains_key(&symbol!("foo")));
                    assert!(symbols.contains_key(&symbol!("another_foo")));
                    assert!(symbols.contains_key(&symbol!("bar")));
                    assert!(symbols.contains_key(&symbol!("baz")));
                    assert!(!symbols.contains_key(&symbol!("random")));
                    assert_eq!(4, symbols.len());
                    assert_eq!(0, *symbols.get(&symbol!("foo")).unwrap());
                    assert_eq!(0, *symbols.get(&symbol!("another_foo")).unwrap());
                    assert_eq!(1, *symbols.get(&symbol!("bar")).unwrap());
                    assert_eq!(2, *symbols.get(&symbol!("baz")).unwrap());
                }
                Ok(StaticLib::DirLib { .. }) => panic!("unexpected StaticLib::DirLib"),
                Err(e) => panic!("build_static_lib_file: {e:?}"),
            }
        }
    }
    ensure_clean_state(&base_loc);
}

#[test]
fn link_with_static_libs() {
    let base_loc = tests_base_loc("link_with_static_libs");
    ensure_clean_state(&base_loc);

    // first build static libs
    let mut librarian = Librarian::new(false);
    let lib_objs = vec!["libmod_1", "libmod_2", "libmod_3"];
    let _ = librarian.build_libdir(Some(&base_loc), None, lib_objs);

    // make sure static libs are built
    let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib"));
    assert!(lib_loc.exists());

    // now link
    let text_start = 0x10;
    let staticlib = read_lib(lib_loc.to_str().unwrap()).unwrap();
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    let mod_names = vec!["mod_1", "mod_2", "mod_3"];
    let objects = read_objects(&base_loc, mod_names);
    match editor.link(objects, vec![staticlib], NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{info:?}");
            assert_eq!(5, info.symbol_tables.len());
            assert_eq!(7, info.global_symtable.len());
            assert!(info.global_symtable.get(&symbol!("malloc")).is_some());
            assert!(info.global_symtable.get(&symbol!("printf")).is_some());
            assert!(info.global_symtable.get(&symbol!("noway")).is_none());
            let text_seg_len = out.segments.get(&SegmentName::TEXT).unwrap().segment_len;
            let data_seg_len = out.segments.get(&SegmentName::DATA).unwrap().segment_len;
            let bss_seg_len = out.segments.get(&SegmentName::BSS).unwrap().segment_len;
            assert_eq!(0x64, text_seg_len);
            assert_eq!(0x2D, data_seg_len);
            assert_eq!(0x14, bss_seg_len);
            assert_eq!(
                text_start + text_seg_len - 0xA - 0x1E, // minus lib_1 and lib_3 text seg lengths
                *info
                    .segment_mapping
                    .get("libmod_1")
                    .unwrap()
                    .get(&SegmentName::TEXT)
                    .unwrap()
            );
            assert_eq!(
                text_start + text_seg_len - 0xA, // minus lib_3 text seg length
                *info
                    .segment_mapping
                    .get("libmod_3")
                    .unwrap()
                    .get(&SegmentName::TEXT)
                    .unwrap()
            );
            ensure_clean_state(&base_loc);
        }
        Err(e) => panic!("link_with_static_libs: {e:?}"),
    }
}

#[test]
fn link_with_static_libs_duplicate_symbol() {
    let base_loc = tests_base_loc("link_with_static_libs_duplicate_symbol");
    ensure_clean_state(&base_loc);

    // first build static libs
    let mut librarian = Librarian::new(false);
    let lib_objs = vec!["libmod_1"];
    let _ = librarian.build_libdir(Some(&base_loc), None, lib_objs);

    // make sure static libs are built
    let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib"));
    assert!(lib_loc.exists());

    // now link
    let text_start = 0x10;
    let staticlib = read_lib(lib_loc.to_str().unwrap()).unwrap();
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    let mod_names = vec!["mod_1"];
    let objects = read_objects(&base_loc, mod_names);
    match editor.link(objects, vec![staticlib], NO_WRAP_ROUTINES) {
        Err(e) => assert_eq!(LinkError::MultipleSymbolDefinitions, e),
        Ok(_) => {
            panic!("link_with_static_libs_duplicate_symbol: unexpected Ok")
        }
    }
}

#[test]
fn link_with_static_libs_lib_deps() {
    let base_loc = tests_base_loc("link_with_static_libs_lib_deps");
    ensure_clean_state_extra(&base_loc, vec!["staticlib1", "staticlib2"]);

    // first build static libs
    let mut librarian = Librarian::new(false);
    let lib1_objs = vec!["libmod_1"];
    let lib2_objs = vec!["liblibmod_1"];
    let _ = librarian.build_libdir(Some(&base_loc), Some("staticlib1"), lib1_objs);
    let _ = librarian.build_libdir(Some(&base_loc), Some("staticlib2"), lib2_objs);

    // make sure static libs are built
    let lib1_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib1"));
    let lib2_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib2"));
    assert!(lib1_loc.exists());
    assert!(lib2_loc.exists());

    // now link
    let text_start = 0x10;
    let staticlib1_dir = read_lib(lib1_loc.to_str().unwrap()).unwrap();
    let staticlib2_dir = read_lib(lib2_loc.to_str().unwrap()).unwrap();
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    let mod_names = vec!["mod_1"];
    let objects = read_objects(&base_loc, mod_names);
    match editor.link(
        objects,
        vec![staticlib1_dir, staticlib2_dir],
        NO_WRAP_ROUTINES,
    ) {
        Ok((_out, info)) => {
            println!("{info:?}");
            assert_eq!(3, info.symbol_tables.len());
            assert_eq!(3, info.global_symtable.len());
            assert!(info.global_symtable.get(&symbol!("exec")).is_some());
            assert!(info.global_symtable.get(&symbol!("printf")).is_some());
            assert!(info.global_symtable.get(&symbol!("nope")).is_none());
        }
        Err(e) => panic!("link_with_static_libs_lib_deps: {e:?}"),
    }
    ensure_clean_state_extra(&base_loc, vec!["staticlib1", "staticlib2"]);
}

#[test]
fn link_with_static_libs_lib_deps_undef() {
    let base_loc = tests_base_loc("link_with_static_libs_lib_deps_undef");
    ensure_clean_state_extra(&base_loc, vec!["staticlib1", "staticlib2"]);

    // first build static libs
    let mut librarian = Librarian::new(false);
    let lib1_objs = vec!["libmod_1"];
    let lib2_objs = vec!["liblibmod_1"];
    let _ = librarian.build_libdir(Some(&base_loc), Some("staticlib1"), lib1_objs);
    let _ = librarian.build_libdir(Some(&base_loc), Some("staticlib2"), lib2_objs);

    // make sure static libs are built
    let lib1_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib1"));
    let lib2_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib2"));
    assert!(lib1_loc.exists());
    assert!(lib2_loc.exists());

    // now link
    let text_start = 0x10;
    let staticlib1_dir = read_lib(lib1_loc.to_str().unwrap()).unwrap();
    let staticlib2_dir = read_lib(lib2_loc.to_str().unwrap()).unwrap();
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    let mod_names = vec!["mod_1"];
    let objects = read_objects(&base_loc, mod_names);
    match editor.link(
        objects,
        vec![staticlib1_dir, staticlib2_dir],
        NO_WRAP_ROUTINES,
    ) {
        Err(e) => assert_eq!(LinkError::UndefinedSymbolError, e),
        Ok(_) => {
            panic!("link_with_static_libs_lib_deps_undef: unexpected Ok")
        }
    }
    ensure_clean_state_extra(&base_loc, vec!["staticlib1", "staticlib2"]);
}

#[test]
fn link_with_static_libs_single_file() {
    let base_loc = tests_base_loc("link_with_static_libs_single_file");
    ensure_clean_state(&base_loc);

    // first build static libs
    let mut librarian = Librarian::new(false);
    let lib_objs = vec!["libmod_1", "libmod_2", "libmod_3"];
    let _ = librarian.build_libfile(Some(&base_loc), None, lib_objs);

    // make sure static libs are built
    let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlibfile"));
    assert!(lib_loc.exists());

    // now link
    let text_start = 0x10;
    let staticlib = read_lib(lib_loc.to_str().unwrap()).unwrap();
    let mut editor = LinkerEditor::new(text_start, 0x10, 0x4, false);
    let mod_names = vec!["mod_1", "mod_2", "mod_3"];
    let objects = read_objects(&base_loc, mod_names);
    match editor.link(objects, vec![staticlib], NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{info:?}");
            assert_eq!(5, info.symbol_tables.len());
            assert_eq!(7, info.global_symtable.len());
            assert!(info.global_symtable.get(&symbol!("malloc")).is_some());
            assert!(info.global_symtable.get(&symbol!("printf")).is_some());
            assert!(info.global_symtable.get(&symbol!("noway")).is_none());
            let text_seg_len = out.segments.get(&SegmentName::TEXT).unwrap().segment_len;
            let data_seg_len = out.segments.get(&SegmentName::DATA).unwrap().segment_len;
            let bss_seg_len = out.segments.get(&SegmentName::BSS).unwrap().segment_len;
            assert_eq!(0x64, text_seg_len);
            assert_eq!(0x2D, data_seg_len);
            assert_eq!(0x14, bss_seg_len);
            assert_eq!(
                text_start + text_seg_len - 0xA - 0x1E, // minus lib_1 and lib_3 text seg lengths
                *info
                    .segment_mapping
                    .get("staticlibfile_mod_0")
                    .unwrap()
                    .get(&SegmentName::TEXT)
                    .unwrap()
            );
            assert_eq!(
                text_start + text_seg_len - 0xA, // minus lib_3 text seg length
                *info
                    .segment_mapping
                    .get("staticlibfile_mod_2")
                    .unwrap()
                    .get(&SegmentName::TEXT)
                    .unwrap()
            );
            ensure_clean_state(&base_loc);
        }
        Err(e) => panic!("link_with_static_libs_single_file: {e:?}"),
    }
}

#[test]
fn run_relocations_a4() {
    let testdir = tests_base_loc("run_relocations_A4");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_text = out.object_data.get(&SegmentName::TEXT).unwrap();
            assert_eq!(
                0x14B,
                x_to_i4(obj_code_text.get_at(0x4, 0x4).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn run_relocations_r4() {
    let testdir = tests_base_loc("run_relocations_R4");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_text = out.object_data.get(&SegmentName::TEXT).unwrap();
            assert_eq!(
                -36,
                x_to_i4(obj_code_text.get_at(0xA, 0x4).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn run_relocations_as4() {
    let testdir = tests_base_loc("run_relocations_AS4");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_text = out.object_data.get(&SegmentName::TEXT).unwrap();
            assert_eq!(
                0xFF,
                x_to_i4(obj_code_text.get_at(0x10, 0x4).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn run_relocation_rs4() {
    let testdir = tests_base_loc("run_relocations_RS4");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_text = out.object_data.get(&SegmentName::TEXT).unwrap();
            assert_eq!(
                -34,
                x_to_i4(obj_code_text.get_at(0x16, 0x4).unwrap()).unwrap()
            );
            assert_eq!(
                34,
                x_to_i4(obj_code_text.get_at(0x28, 0x4).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn run_relocation_u2() {
    let testdir = tests_base_loc("run_relocations_U2");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_data = out.object_data.get(&SegmentName::DATA).unwrap();
            assert_eq!(0, x_to_i2(obj_code_data.get_at(0x0, 0x2).unwrap()).unwrap());
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn run_relocation_l2() {
    let testdir = tests_base_loc("run_relocations_L2");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0xFF, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_data = out.object_data.get(&SegmentName::DATA).unwrap();
            assert_eq!(
                0x011D,
                x_to_i2(obj_code_data.get_at(0x8, 0x2).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn wrap_routine() {
    let testdir = tests_base_loc("wrap_routine");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0x0, 0x0, 0x0, false);
    let wrap_routines = vec![symbol!("foo")];
    match editor.link(objects, NO_STATIC_LIBS, wrap_routines) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            assert!(info.global_symtable.contains_key(&wrapped_symbol!("foo")));
            assert!(!info.global_symtable.contains_key(&wrapped_symbol!("bar")));
            assert!(!info.global_symtable.contains_key(&symbol!("foo")));
            assert!(info.global_symtable.contains_key(&symbol!("bar")));
            assert!(editor
                .session_objects
                .get("mod_1")
                .unwrap()
                .ppr(false)
                .contains("real_foo"));
            assert!(!editor
                .session_objects
                .get("mod_1")
                .unwrap()
                .ppr(false)
                .contains("wrap_foo"));
            assert!(editor
                .session_objects
                .get("mod_1")
                .unwrap()
                .ppr(false)
                .contains("bar"));
            assert!(!editor
                .session_objects
                .get("mod_2")
                .unwrap()
                .ppr(false)
                .contains("real_bar"));
            assert!(!editor
                .session_objects
                .get("mod_2")
                .unwrap()
                .ppr(false)
                .contains("real_foo"));
            assert!(editor
                .session_objects
                .get("mod_2")
                .unwrap()
                .ppr(false)
                .contains("wrap_foo"));
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}

#[test]
fn wrap_routine_error() {
    let testdir = tests_base_loc("wrap_routine_error");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0x0, 0x0, 0x0, false);
    let wrap_routines = vec![symbol!("foo")];
    match editor.link(objects, NO_STATIC_LIBS, wrap_routines) {
        Err(e) => assert_eq!(LinkError::WrappedSymbolNameAlreadyExists, e),
        Ok(_) => panic!("wrap_routine_error unexpected OK"),
    }
}

#[test]
fn position_independent_code() {
    let testdir = tests_base_loc("position_independent_code");
    let objects = read_objects_from_dir(&testdir);
    let mut editor = LinkerEditor::new(0x0, 0x0, 0x0, false);
    match editor.link(objects, NO_STATIC_LIBS, NO_WRAP_ROUTINES) {
        Ok((out, info)) => {
            println!("{out:?}");
            println!("{info:?}");
            let obj_code_text = out.object_data.get(&SegmentName::TEXT).unwrap();
            assert_eq!(
                0x24,
                x_to_i4(obj_code_text.get_at(0x8, 0x4).unwrap()).unwrap()
            );
            assert_eq!(8, out.object_data.get(&SegmentName::GOT).unwrap().len());
            assert_eq!(
                0x1C,
                x_to_i4(obj_code_text.get_at(0xC, 0x4).unwrap()).unwrap()
            );
        }
        Err(e) => panic!("{testdir} {e:?}"),
    }
}
