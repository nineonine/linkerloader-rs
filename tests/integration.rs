use std::ops::Deref;
use linkerloader::types::errors::ParseError;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::types::segment::{SegmentName, SegmentDescr};
use linkerloader::types::symbol_table::{STE, SymbolTableEntryType};
use linkerloader::types::relocation::{Relocation, RelRef, RelType};
use linkerloader::lib::read_object;
use linkerloader::utils::read_object_file;

const TESTS_DIR: &'static str = "tests/input/";

#[test]
fn test_magic_number_simple() {
    let obj_file = read_object_file("tests/input/simple");
    let magic_number = obj_file.lines().next().unwrap();
    assert_eq!(MAGIC_NUMBER, magic_number);
}

fn test_failure(e0: ParseError, fp: &str) {
    let res = read_object(fp);
    if res.is_ok() {
        println!("{:?}", res);
        assert!(res.is_err());
    }
    match res {
        Ok(_) => {
            panic!("unexpected");
        },
        Err(e) => assert_eq!(e0, e),
    }
}

fn tests_base_loc(filename: &str) -> String {
    format!("{}{}", TESTS_DIR, filename)
}

#[test]
fn magic_number_not_present() {
    test_failure(ParseError::MissingMagicNumber, &tests_base_loc("no_magic_number"));
}

#[test]
fn invalid_magic_number() {
    test_failure(ParseError::InvalidMagicNumber, &tests_base_loc("invalid_magic_number"));
}

#[test]
fn missing_nsegs_nsums_nrels() {
    test_failure(ParseError::MissingNSegsNSumsNRels, &tests_base_loc("missing_nsegs_nsums_nrels"));
}

#[test]
fn invalid_nsegs_nsums_nrels() {
    test_failure(ParseError::InvalidNSegsNSumsNRels, &tests_base_loc("invalid_nsegs_nsums_nrels"));
}

#[test]
fn invalid_nsegs() {
    test_failure(ParseError::InvalidNSegsValue, &tests_base_loc("invalid_nsegs"));
}

#[test]
fn invalid_nsyms() {
    test_failure(ParseError::InvalidNSymsValue, &tests_base_loc("invalid_nsyms"));
}

#[test]
fn invalid_nrels() {
    test_failure(ParseError::InvalidNRelsValue, &tests_base_loc("invalid_nrels"));
}

#[test]
fn invalid_segment_name() {
    test_failure(ParseError::InvalidSegmentName, &tests_base_loc("invalid_segment_name"));
}

#[test]
fn invalid_segment_start() {
    test_failure(ParseError::InvalidSegmentStart, &tests_base_loc("invalid_segment_start"));
}

#[test]
fn invalid_segment_len() {
    test_failure(ParseError::InvalidSegmentLen, &tests_base_loc("invalid_segment_len"));
}

#[test]
fn invalid_segment_descr() {
    test_failure(ParseError::InvalidSegmentDescr, &tests_base_loc("invalid_segment_descr"));
}

#[test]
fn invalid_num_of_segs_1() {
    test_failure(ParseError::InvalidNumOfSegments, &tests_base_loc("invalid_num_of_segs_1"));
}

#[test]
fn invalid_num_of_segs_2() {
    test_failure(ParseError::InvalidNumOfSegments, &tests_base_loc("invalid_num_of_segs_2"));
}

#[test]
fn invalid_num_of_segs_3() {
    test_failure(ParseError::InvalidNumOfSegments, &tests_base_loc("invalid_num_of_segs_3"));
}

#[test]
fn invalid_num_of_segs_4() {
    test_failure(ParseError::InvalidNumOfSegments, &tests_base_loc("invalid_num_of_segs_4"));
}

#[test]
fn segments() {
    let res = read_object(&tests_base_loc("segments_1"));
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
    test_failure(ParseError::InvalidSymbolTableEntry, &tests_base_loc("invalid_symbol_table_entry"));
}

#[test]
fn invalid_symbol_table_entry_seg() {
    test_failure(ParseError::InvalidSTESegment, &tests_base_loc("invalid_symbol_table_entry_seg"));
}

#[test]
fn invalid_symbol_table_type() {
    test_failure(ParseError::InvalidSTEType, &tests_base_loc("invalid_symbol_table_entry_type"));
}

#[test]
fn invalid_symbol_table_value() {
    test_failure(ParseError::InvalidSTEValue, &tests_base_loc("invalid_symbol_table_entry_value"));
}

#[test]
fn invalid_symbol_table_segment_out_of_range() {
    test_failure(ParseError::STESegmentRefOutOfRange, &tests_base_loc("invalid_symbol_table_seg_out_of_range"));
}

#[test]
fn non_zero_segment_for_undefined_ste() {
    test_failure(ParseError::NonZeroSegmentForUndefinedSTE, &tests_base_loc("non_zero_segment_for_undefined_ste"));
}

#[test]
fn symbol_table() {
    let res = read_object(&tests_base_loc("symbol_table_1"));
    println!("{:?}", res);
    assert!(res.is_ok());
    match res {
        Err(_) => panic!("unexpected"),
        Ok(obj) => {
            assert_eq!(obj.nsyms, obj.symbol_table.len() as i32);
            let ste1: &STE = &obj.symbol_table[0];
            assert_eq!("foo", ste1.st_name);
            assert_eq!(0x1a, ste1.st_value);
            assert_eq!(1, ste1.st_seg); // 2500 decimal
            assert_eq!(SymbolTableEntryType::D, ste1.st_type);
            let ste2: &STE= &obj.symbol_table[1];
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
    test_failure(ParseError::InvalidRelocationEntry, &tests_base_loc("invalid_reloc_entry"));
}

#[test]
fn invalid_relocation_addr() {
    test_failure(ParseError::InvalidRelRef, &tests_base_loc("invalid_reloc_addr"));
}

#[test]
fn rel_segment_out_of_range() {
    test_failure(ParseError::RelSegmentOutOfRange, &tests_base_loc("reloc_segment_out_of_range"));
}

#[test]
fn rel_symbol_out_of_range() {
    test_failure(ParseError::RelSymbolOutOfRange, &tests_base_loc("reloc_symbol_out_of_range"));
}

#[test]
fn invalid_reloc_type() {
    test_failure(ParseError::InvalidRelType, &tests_base_loc("invalid_reloc_type"));
}

#[test]
fn invalid_reloc_segment() {
    test_failure(ParseError::InvalidRelSegment, &tests_base_loc("invalid_reloc_segment"));
}

#[test]
fn invalid_num_of_relocations_1() {
    test_failure(ParseError::InvalidNumOfRelocations, &tests_base_loc("invalid_num_of_relocations_1"));
}

#[test]
fn invalid_num_of_relocations_2() {
    test_failure(ParseError::InvalidNumOfRelocations, &tests_base_loc("invalid_num_of_relocations_2"));
}

#[test]
fn relocations() {
    let res = read_object(&tests_base_loc("relocations_1"));
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
            let rel2: &Relocation= &obj.relocations[1];
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
    test_failure(ParseError::InvalidObjectData, &tests_base_loc("invalid_object_data"));
}

#[test]
fn segment_data_len_mismatch() {
    test_failure(ParseError::SegmentDataLengthMismatch, &tests_base_loc("segment_data_len_mismatch"));
}

#[test]
fn segment_data_out_of_bounds() {
    test_failure(ParseError::SegmentDataOutOfBounds, &tests_base_loc("segment_data_out_of_bounds"));
}
