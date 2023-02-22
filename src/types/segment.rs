use crate::types::errors::ParseError;
use std::fmt;
use std::ops::Deref;

// Each segment definition contains the
// segment name, the address where the segment logically starts, the length
// of the segment in bytes, and a string of code letters describing the segment.
// Code letters include R for readable, W for writable, and P for present in the
// object file. (Other letters may be present as well.) A typical set of segments
// for an a.out like file would be:
//   .text 1000 2500 RP
//   .data 4000 C00 RWP
//   .bss 5000 1900 RW
// Segments are numbered in the order their definitions appear, with the first
// segment being number 1.
#[derive(Debug, Clone)]
pub struct Segment {
    pub segment_name: SegmentName,
    pub segment_start: i32,
    pub segment_len: i32,                 // bytes
    pub segment_descr: Vec<SegmentDescr>, // TODO: ensure uniqueness when parsing
}

impl Segment {
    pub fn new(segment_name: SegmentName) -> Segment {
        Segment {
            segment_name,
            segment_start: 0,
            segment_len: 0,
            segment_descr: vec![],
        }
    }

    pub fn ppr_seg_descr(&self) -> String {
        self.segment_descr
            .iter()
            .map(|sd| match sd {
                SegmentDescr::R => "R",
                SegmentDescr::W => "W",
                SegmentDescr::P => "P",
            })
            .collect::<Vec<&str>>()
            .join("")
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Ord, PartialOrd)]
pub enum SegmentName {
    TEXT,
    GOT,
    DATA,
    BSS,
}

impl fmt::Display for SegmentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segment_name_str = match self {
            SegmentName::TEXT => ".text",
            SegmentName::GOT => ".got",
            SegmentName::DATA => ".data",
            SegmentName::BSS => ".bss",
        };
        write!(f, "{segment_name_str}")
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SegmentDescr {
    R, // readable
    W, // writable
    P, // present in the object file
}

#[derive(Debug, Clone)]
pub struct SegmentData(Vec<u8>);
impl Deref for SegmentData {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SegmentData {
    pub fn new(len: usize) -> Self {
        SegmentData(vec![0; len])
    }

    pub fn concat(&self, other: &SegmentData) -> SegmentData {
        let mut new_vec = self.0.clone();
        new_vec.extend_from_slice(&other.0);
        SegmentData(new_vec)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn update(&mut self, start: usize, len: usize, patch: Vec<u8>) {
        let end = start + len;
        let data_len = self.len();
        if end > data_len {
            panic!("Index out of bounds: end {end} is greater than data length {data_len}");
        }
        let mut new_data = Vec::with_capacity(data_len + patch.len());
        new_data.extend_from_slice(&self.0[0..start]);
        new_data.extend_from_slice(&patch);
        new_data.extend_from_slice(&self.0[end..data_len]);

        self.0 = new_data;
    }

    pub fn get_at(&self, start: usize, len: usize) -> Option<&[u8]> {
        let end = start + len;
        if end > self.0.len() {
            return None;
        }
        Some(&self.0[start..end])
    }
}

pub fn parse_segment(s: &str) -> Result<Segment, ParseError> {
    let segment_name;
    let segment_start;
    let segment_len;
    let segment_descr;
    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [name, start, len, descr] => {
            match *name {
                ".text" => segment_name = SegmentName::TEXT,
                ".data" => segment_name = SegmentName::DATA,
                ".bss" => segment_name = SegmentName::BSS,
                _ => return Err(ParseError::InvalidSegmentName),
            }
            match i32::from_str_radix(start, 16) {
                Err(_) => return Err(ParseError::InvalidSegmentStart),
                Ok(i) => segment_start = i,
            }
            match i32::from_str_radix(len, 16) {
                Err(_) => return Err(ParseError::InvalidSegmentLen),
                Ok(i) => segment_len = i,
            }
            let mut descrs: Vec<SegmentDescr> = vec![];
            for c in descr.chars() {
                match segment_descr_from_chr(c) {
                    Err(e) => return Err(e),
                    Ok(sd) => descrs.push(sd),
                }
            }
            segment_descr = descrs;
        }
        _otherwise => return Err(ParseError::InvalidSegment),
    }

    Ok(Segment {
        segment_name,
        segment_start,
        segment_len,
        segment_descr,
    })
}

fn segment_descr_from_chr(c: char) -> Result<SegmentDescr, ParseError> {
    match c {
        'R' => Ok(SegmentDescr::R),
        'W' => Ok(SegmentDescr::W),
        'P' => Ok(SegmentDescr::P),
        _ => Err(ParseError::InvalidSegmentDescr),
    }
}

pub fn parse_segment_data(seg_len: usize, s: &str) -> Result<SegmentData, ParseError> {
    let x: Vec<u8> = s
        .split_whitespace()
        .map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect();
    if x.len() != seg_len {
        Err(ParseError::SegmentDataLengthMismatch)
    } else {
        Ok(SegmentData(x))
    }
}
