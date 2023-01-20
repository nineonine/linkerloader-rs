use std::collections::HashMap;

use crate::types::object::ObjectIn;
use crate::types::out::ObjectOut;

pub struct LinkerEditor {
    text_start: i32,
    bss_start_boundary: i32,
    data_start: i32,
}

type ObjectID = String;

impl LinkerEditor {

    pub fn link(&mut self, objects: HashMap<ObjectID, ObjectIn>) -> ObjectOut {
        ObjectOut::new()
    }
}
