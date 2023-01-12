use std::fs;

pub fn read_object_file(file_path: &str) -> String {
    let contents = fs::read_to_string(file_path).expect("Failed to read object file");
    contents
}
