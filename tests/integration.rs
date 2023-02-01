use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
// use linkerloader::gen::gen_obj_data;
use linkerloader::lib::{parse_object, read_lib, read_objects_from_dir};
use linkerloader::librarian::Librarian;
use linkerloader::linker::editor::LinkerEditor;
use linkerloader::types::errors::{LinkError, ParseError};
use linkerloader::types::library::StaticLib;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::types::relocation::{RelRef, RelType, Relocation};
use linkerloader::types::segment::{SegmentDescr, SegmentName};
use linkerloader::types::symbol_table::{SymbolTableEntry, SymbolTableEntryType};
use linkerloader::utils::read_object_file;

const TESTS_DIR: &'static str = "tests/input/";

fn ensure_clean_state(path: &str) {
    println!("test cleanup");
    let p = PathBuf::from(path);
    if p.exists() {
        // delete static lib if exists
        let static_lib = p.join(PathBuf::from("staticlib"));
        if static_lib.exists() {
            println!("removing static lib");
            fs::remove_dir_all(static_lib).unwrap();
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
    match editor.link(objects) {
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

fn tests_base_loc(filename: &str) -> String {
    format!("{}{}", TESTS_DIR, filename)
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
            assert_eq!("foo", ste1.st_name);
            assert_eq!(0x1a, ste1.st_value);
            assert_eq!(1, ste1.st_seg); // 2500 decimal
            assert_eq!(SymbolTableEntryType::D, ste1.st_type);
            let ste2: &SymbolTableEntry = &obj.symbol_table[1];
            assert_eq!("bas", ste2.st_name);
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
            assert_eq!(RelRef::SymbolRef(1), rel1.rel_ref);
            assert_eq!(RelType::R(4), rel1.rel_type);
            let rel2: &Relocation = &obj.relocations[1];
            assert_eq!(0x1a, rel2.rel_loc);
            assert_eq!(SegmentName::TEXT, rel2.rel_seg);
            assert_eq!(RelRef::SymbolRef(2), rel2.rel_ref);
            assert_eq!(RelType::R(4), rel2.rel_type);
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
    match editor.link(objects) {
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
    match editor.link(objects) {
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
    match editor.link(objects) {
        Ok((_out, info)) => {
            assert_eq!(2, info.global_symtable.len());
            assert!(info.global_symtable.contains_key("foo"));
            assert!(info.global_symtable.contains_key("bar"));
            let foo_ste = info.global_symtable.get("foo").unwrap().clone();
            assert_eq!("mod_2", foo_ste.0.as_ref().unwrap().0);
            assert_eq!(0, foo_ste.0.as_ref().unwrap().1);
            assert!(foo_ste.1.contains_key("mod_1"));
            assert_eq!(0, *foo_ste.1.get("mod_1").unwrap());
            let bar_ste = info.global_symtable.get("bar").unwrap().clone();
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
    match editor.link(objects) {
        Err(e) => assert_eq!(LinkError::MultipleSymbolDefinitions, e),
        _ => panic!("{}", dirname),
    }
}

#[test]
fn undefined_symbol() {
    let dirname = "undefined_symbol";
    let objects = read_objects_from_dir(&tests_base_loc(dirname));
    let mut editor = LinkerEditor::new(0x10, 0x10, 0x4, false);
    match editor.link(objects) {
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
    match editor.link(objects) {
        Ok((_out, info)) => {
            println!("{:?}", info);
            assert_eq!(3, info.global_symtable.len());
            let foo_abs_addr = info
                .global_symtable
                .get("foo")
                .unwrap()
                .0
                .clone()
                .unwrap()
                .2
                .unwrap();
            assert_eq!(0x20, foo_abs_addr);
            let bar_abs_addr = info
                .global_symtable
                .get("bar")
                .unwrap()
                .0
                .clone()
                .unwrap()
                .2
                .unwrap();
            assert_eq!(0x5A + 0x5, bar_abs_addr);
            let baz_abs_addr = info
                .global_symtable
                .get("baz")
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
            assert!(symbols.get("libmod_1").unwrap().contains("foo"));
            assert!(symbols.get("libmod_1").unwrap().contains("another_foo"));
            assert!(symbols.contains_key("libmod_2"));
            assert!(symbols.get("libmod_2").unwrap().contains("bar"));
            assert!(symbols.contains_key("libmod_3"));
            assert!(symbols.get("libmod_3").unwrap().contains("baz"));
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
            assert_eq!(0, *symbols.get("foo").unwrap());
            assert_eq!(0, *symbols.get("another_foo").unwrap());
            assert_eq!(1, *symbols.get("bar").unwrap());
            assert_eq!(2, *symbols.get("baz").unwrap());
        }
        Ok(StaticLib::DirLib { .. }) => panic!("unexpected StaticLib::DirLib"),
        Err(e) => panic!("{}: {:?}", dirname, e),
    }
}

#[test]
fn build_static_lib() {
    let base_loc = tests_base_loc("build_static_lib_dir");
    ensure_clean_state(&base_loc);
    let objs = vec!["libmod_1", "libmod_2", "libmod_3"];
    let mut librarian = Librarian::new(false);
    match librarian.build_dir(Some(&base_loc), None, objs) {
        Err(_) => panic!("build_static_lib"),
        Ok(_) => {
            let lib_loc = PathBuf::from(&base_loc).join(PathBuf::from("staticlib"));
            assert!(lib_loc.exists());
            match read_lib(lib_loc.to_str().unwrap()) {
                Ok(StaticLib::DirLib { symbols, .. }) => {
                    assert_eq!(3, symbols.len());
                    assert!(symbols.contains_key("libmod_1"));
                    assert!(symbols.get("libmod_1").unwrap().contains("foo"));
                    assert!(symbols.get("libmod_1").unwrap().contains("another_foo"));
                    assert!(symbols.contains_key("libmod_2"));
                    assert!(symbols.get("libmod_2").unwrap().contains("bar"));
                    assert!(symbols.contains_key("libmod_3"));
                    assert!(symbols.get("libmod_3").unwrap().contains("baz"));
                }
                Ok(StaticLib::FileLib { .. }) => panic!("unexpected StaticLib::FileLib"),
                Err(_) => panic!("build_static_lib"),
            }
        }
    }
    ensure_clean_state(&base_loc);
}
