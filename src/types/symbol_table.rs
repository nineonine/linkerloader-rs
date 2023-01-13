// Symbol table. Each entry is of the form:
//   name value seg type
// The name is the symbol name. The value is the hex value of the symbol.
// Seg is the segment number relative to which the segment is defined, or 0
// for absolute or undefined symbols. The type is a string of letters including
// D for defined or U for undefined. Symbols are also numbered in the order
// they’re listed, starting at 1.
pub struct SymbolTable {
    pub st_name: String,
    pub st_value: i32,
    pub st_seg: i32,
    pub st_type: SymbolTableType,
}

pub enum SymbolTableType {
    D, // defined
    U, // undefined
}
