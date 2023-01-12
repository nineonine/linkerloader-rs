pub mod utils;
pub mod types;

pub mod lib {

    use crate::types::object::{ObjectFile,parse_object_file};
    use crate::types::errors::ParseError;
    use crate::utils::read_object_file;

    pub fn read_object(fp: &str) -> Result<ObjectFile, ParseError> {
        let file_contents = read_object_file(fp);
        return parse_object_file(file_contents);
    }
}
