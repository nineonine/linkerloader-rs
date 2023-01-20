pub mod utils;
pub mod types;
pub mod gen;
pub mod logger;
pub mod linker;

pub mod lib {

    use crate::types::object::{ObjectIn,parse_object_file};
    use crate::types::errors::ParseError;
    use crate::utils::read_object_file;

    pub fn parse_object(fp: &str) -> Result<ObjectIn, ParseError> {
        let file_contents = read_object_file(fp);
        return parse_object_file(file_contents);
    }
}
