use crate::types::segment;

pub struct ObjectFile {
    nsegs: Option<i32>,
    nsyms: Option<i32>,
    nrels: Option<i32>,
}
