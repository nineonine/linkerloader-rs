pub struct SymbolTable {
    st_name: String,
    st_value: i32,
    st_seg: i32,
    st_type: SymbolTableType,
}

pub enum SymbolTableType {
    D, // defined
    U, // undefined
}
