use std::fs;

pub fn read_object_file(file_path: &str) -> String {
    let contents = fs::read_to_string(file_path).expect("Failed to read object file");
    contents
}

pub fn find_seg_start(i: i32, n: i32) -> i32 {
    if n == 0 {
        return i;
    }
    let rem = i % n;
    if rem == 0 {
        i
    } else {
        i + (n - rem)
    }
}
