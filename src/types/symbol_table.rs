use std::{fmt, ops::Deref};

use super::errors::ParseError;

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Hash, Debug)]
pub enum SymbolName {
    SName(String),
    WrappedSName(String),
}

impl fmt::Display for SymbolName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolName::SName(s) => write!(f, "{s}"),
            SymbolName::WrappedSName(s) => write!(f, "wrap_{s}"),
        }
    }
}

impl Deref for SymbolName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        match self {
            SymbolName::SName(s) => s,
            SymbolName::WrappedSName(s) => s,
        }
    }
}

#[macro_export]
macro_rules! symbol {
    ($name:expr) => {
        SymbolName::SName(String::from($name))
    };
}

#[macro_export]
macro_rules! wrapped_symbol {
    ($name:expr) => {
        SymbolName::WrappedSName(String::from($name))
    };
}

// Symbol table entry. Each entry is of the form:
//   name value seg type
// The name is the symbol name. The value is the hex value of the symbol.
// Seg is the segment number relative to which the symbol is defined, or 0
// for absolute or undefined symbols. The type is a string of letters including
// D for defined or U for undefined. Symbols are also numbered in the order
// they are listed, starting at 1.
#[derive(Debug, Clone)]
pub struct SymbolTableEntry {
    pub st_name: SymbolName,
    pub st_value: i32, // for local defined symbols - segment offset
    // for common blocks - size to be appened to BSS
    // for global undefined symbols - always zero
    pub st_seg: i32,
    pub st_type: SymbolTableEntryType,
}

impl SymbolTableEntry {
    pub fn is_common_block(&self) -> bool {
        if self.st_type == SymbolTableEntryType::U && self.st_value != 0 {
            return true;
        }
        false
    }

    pub fn is_defined(&self) -> bool {
        self.st_type == SymbolTableEntryType::D
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SymbolTableEntryType {
    D, // defined
    U, // undefined
}

impl fmt::Display for SymbolTableEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segment_name_str = match self {
            SymbolTableEntryType::D => "D",
            SymbolTableEntryType::U => "U",
        };
        write!(f, "{segment_name_str}")
    }
}

pub fn parse_symbol_table_entry(nsegs: i32, s: &str) -> Result<SymbolTableEntry, ParseError> {
    let st_name;
    let st_value;
    let st_seg;
    let st_type;

    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [name, value, seg, ty] => {
            st_name = SymbolName::SName(String::from(*name));
            match i32::from_str_radix(value, 16) {
                Err(_) => return Err(ParseError::InvalidSTEValue),
                Ok(i) => st_value = i,
            }
            match i32::from_str_radix(seg, 16) {
                Err(_) => return Err(ParseError::InvalidSTESegment),
                Ok(i) => {
                    if i > nsegs {
                        return Err(ParseError::STESegmentRefOutOfRange);
                    }
                    st_seg = i;
                }
            }
            match *ty {
                "D" => st_type = SymbolTableEntryType::D,
                "U" => st_type = SymbolTableEntryType::U,
                _ => return Err(ParseError::InvalidSTEType),
            }
        }
        _otherwise => return Err(ParseError::InvalidSymbolTableEntry),
    }

    Ok(SymbolTableEntry {
        st_name,
        st_value,
        st_seg,
        st_type,
    })
}
