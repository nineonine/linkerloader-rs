use super::errors::ParseError;

// Symbol table entry. Each entry is of the form:
//   name value seg type
// The name is the symbol name. The value is the hex value of the symbol.
// Seg is the segment number relative to which the segment is defined, or 0
// for absolute or undefined symbols. The type is a string of letters including
// D for defined or U for undefined. Symbols are also numbered in the order
// they are listed, starting at 1.
#[derive(Debug)]
pub struct STE {
    pub st_name: String,
    pub st_value: i32,
    pub st_seg: i32,
    pub st_type: SymbolTableEntryType,
}

pub fn parse_symbol_table_entry(nsegs: i32, s: &str) -> Result<STE, ParseError> {
    let st_name;
    let st_value;
    let st_seg;
    let st_type;

    let vs: Vec<&str> = s.split_ascii_whitespace().collect();
    match vs.as_slice() {
        [name, value, seg, ty] => {
            st_name = String::from(*name);
            match i32::from_str_radix(value, 16) {
                Err(_) => return Err(ParseError::InvalidSTEValue),
                Ok(i) => st_value = i,
            }
            match i32::from_str_radix(seg, 16) {
                Err(_) => return Err(ParseError::InvalidSTESegment),
                Ok(i) => {
                    if i > nsegs { return Err(ParseError::STESegmentRefOutOfRange) }
                    st_seg = i;
                }
            }
            match *ty {
                "D" => st_type = SymbolTableEntryType::D,
                "U" => st_type = SymbolTableEntryType::U,
                _ => return Err(ParseError::InvalidSTEType)
            }
            if st_type == SymbolTableEntryType::U && st_seg != 0 {
                return Err(ParseError::NonZeroSegmentForUndefinedSTE);
            }
        },
        _otherwise => return Err(ParseError::InvalidSymbolTableEntry)
    }

    Ok(STE{
        st_name,
        st_value,
        st_seg,
        st_type
    })
}

#[derive(Debug, Eq, PartialEq)]
pub enum SymbolTableEntryType {
    D, // defined
    U, // undefined
}
