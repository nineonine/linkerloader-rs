use std::fs;

pub fn read_object_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect("Failed to read object file")
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

pub fn range_pairs(array: &[i32]) -> Vec<(i32, i32)> {
    let mut result = Vec::new();
    for window in array.windows(2) {
        result.push((window[0], window[1] - 1));
    }
    result
}

pub fn count_new_lines(s: &str) -> usize {
    s.chars().filter(|&c| c == '\n').count()
}
