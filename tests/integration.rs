use linkerloader::types::errors::ParseError;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::types::segment::{SegmentName, SegmentDescr};
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
    assert!(res.is_err());
    match res {
        Ok(_) => panic!("unexpected"),
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
    test_failure(ParseError::InvalidNumOfSegments, &tests_base_loc("invalid_num_of_segs_3"));
}

#[test]
fn segments() {
    let res = read_object(&tests_base_loc("segments_1"));
    assert!(res.is_ok());
    match res {
        Err(_) => panic!("unexpected"),
        Ok(obj) => {
            assert_eq!(obj.nsegs, obj.segments.len() as i32);
            let seg1 = &obj.segments[0];
            assert_eq!(SegmentName::TEXT, seg1.segment_name);
            assert_eq!(4096, seg1.segment_start); // 1000 decimal
            assert_eq!(9472, seg1.segment_len); // 2500 decimal
            assert_eq!(SegmentDescr::R, seg1.segment_descr[0]);
            assert_eq!(SegmentDescr::P, seg1.segment_descr[1]);
        }
    }
}
