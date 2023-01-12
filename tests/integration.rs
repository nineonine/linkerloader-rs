use linkerloader::utils::read_object_file;
use linkerloader::lib::MAGIC_NUMBER;

#[test]
fn test_magic_number_simple() {
    let obj_file = read_object_file("tests/input/simple");
    let magic_number = obj_file.lines().next().unwrap();
    assert_eq!(MAGIC_NUMBER, magic_number);
}
