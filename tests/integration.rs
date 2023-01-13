use linkerloader::utils::read_object_file;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::lib::read_object;
use linkerloader::types::errors::ParseError;

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
fn test_magic_number_not_present() {
    test_failure(ParseError::MissingMagicNumber, &tests_base_loc("no_magic_number"));
}

#[test]
fn test_invalid_magic_number() {
    test_failure(ParseError::InvalidMagicNumber, &tests_base_loc("invalid_magic_number"));
}

#[test]
fn test_missing_nsegs_nsums_nrels() {
    test_failure(ParseError::MissingNSegsNSumsNRels, &tests_base_loc("missing_nsegs_nsums_nrels"));
}

#[test]
fn test_invalid_nsegs_nsums_nrels() {
    test_failure(ParseError::InvalidNSegsNSumsNRels, &tests_base_loc("invalid_nsegs_nsums_nrels"));
}

#[test]
fn test_invalid_nsegs() {
    test_failure(ParseError::InvalidNSegsValue, &tests_base_loc("invalid_nsegs"));
}

#[test]
fn test_invalid_nsyms() {
    test_failure(ParseError::InvalidNSymsValue, &tests_base_loc("invalid_nsyms"));
}

#[test]
fn test_invalid_nrels() {
    test_failure(ParseError::InvalidNRelsValue, &tests_base_loc("invalid_nrels"));
}
