use linkerloader::utils::read_object_file;
use linkerloader::types::object::MAGIC_NUMBER;
use linkerloader::lib::read_object;
use linkerloader::types::errors::ParseError;

#[test]
fn test_magic_number_simple() {
    let obj_file = read_object_file("tests/input/simple");
    let magic_number = obj_file.lines().next().unwrap();
    assert_eq!(MAGIC_NUMBER, magic_number);
}

#[test]
fn test_magic_number_not_present() {
    let res = read_object("tests/input/no_magic_number");
    assert!(res.is_err());
    match res {
        Ok(_) => panic!("test_magic_number_not_present: unexpected"),
        Err(e) => assert_eq!(e, ParseError::MissingMagicNumber),
    }
}

#[test]
fn test_invalid_magic_number() {
    let res = read_object("tests/input/invalid_magic_number");
    assert!(res.is_err());
    match res {
        Ok(_) => panic!("test_invalid_magic_number: unexpected"),
        Err(e) => assert_eq!(e, ParseError::InvalidMagicNumber),
    }
}
