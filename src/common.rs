use std::collections::HashMap;

pub const MAP_FILE_NAME: &str = "MAP";
pub const MAGIC_NUMBER_LIB: &str = "LIBRARY";
pub const STUB_MAGIC_NUMBER: &str = "STUB";
pub const LIB_NAME_FILE: &str = "LIBRARY NAME";
pub const SHARED_LIBS_SYMBOL: &str = "_SHARED_LIBRARIES";

pub type LibName = String;
pub type StubMemberName = String;
pub type ObjectID = String;

pub type Address = i32;

#[derive(Debug, Clone)]
pub enum DefnProvenance {
    FromObjectIn,
    FromSharedLib(LibName),
}
#[derive(Debug, Clone)]
pub struct Defn {
    pub defn_mod_id: ObjectID,
    pub defn_ste_ix: Option<usize>, // None for shared libs
    pub defn_addr: Option<i32>,
    pub defn_prov: DefnProvenance,
}

impl Defn {
    pub fn new(defn_mod_id: ObjectID, ste_ix: usize, defn_addr: Option<i32>) -> Self {
        Defn {
            defn_mod_id,
            defn_ste_ix: Some(ste_ix),
            defn_addr,
            defn_prov: DefnProvenance::FromObjectIn,
        }
    }

    pub fn shared_lib_defn(defn_mod_id: ObjectID, addr: i32, libname: LibName) -> Self {
        Defn {
            defn_mod_id,
            defn_ste_ix: None,
            defn_addr: Some(addr),
            defn_prov: DefnProvenance::FromSharedLib(libname),
        }
    }
}
pub type Refs = HashMap<ObjectID, usize>;
