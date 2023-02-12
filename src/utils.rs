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

pub fn mk_addr_4(i: i32) -> Option<Vec<u8>> {
    if !(0..=0xFFFF).contains(&i) {
        return None;
    } // we only support width of 4
    let s = format!("{i:04X}");
    let mut result = vec![];

    for i in (0..s.len()).step_by(2) {
        let pair = &s[i..i + 2];
        let byte = u8::from_str_radix(pair, 16).unwrap();
        result.push(byte);
    }

    Some(result)
}

pub fn addr_4_to_i(bytes: &[u8]) -> Option<i32> {
    if bytes.len() != 2 {
        return None;
    }

    let hex_string = format!("{:02X}{:02X}", bytes[0], bytes[1]);
    let result = i32::from_str_radix(&hex_string, 16).unwrap();
    Some(result)
}
