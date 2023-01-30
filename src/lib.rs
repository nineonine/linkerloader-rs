pub mod gen;
pub mod linker;
pub mod logger;
pub mod types;
pub mod utils;

pub mod lib {

    use crate::types::errors::ParseError;
    use crate::types::object::{parse_object_file, ObjectIn};
    use crate::utils::read_object_file;

    pub fn parse_object(fp: &str) -> Result<ObjectIn, ParseError> {
        let file_contents = read_object_file(fp);
        return parse_object_file(file_contents);
    }
}
