pub mod common;
pub mod gen;
pub mod librarian;
pub mod linker;
pub mod logger;
pub mod types;
pub mod utils;

pub mod lib {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    use crate::types::errors::{LibError, ParseError};
    use crate::types::library::StaticLib;
    use crate::types::object::{parse_object_file, ObjectIn};
    use crate::utils::read_object_file;

    type ObjectName = String;

    pub fn parse_object(fp: &str) -> Result<ObjectIn, ParseError> {
        let file_contents = read_object_file(fp);
        parse_object_file(file_contents)
    }

    pub fn read_objects_from_dir(dirname: &str) -> BTreeMap<ObjectName, ObjectIn> {
        let mut objects = BTreeMap::new();
        let mut entries = fs::read_dir(dirname)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_file()
                && !path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .ends_with("_out")
            {
                let file_contents = fs::read_to_string(&path).unwrap();
                let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                println!("reading {}", file_name.as_str());
                match parse_object_file(file_contents) {
                    Ok(object) => {
                        objects.insert(file_name, object);
                    }
                    Err(err) => panic!("read_objects_from_dir: {err:?}"),
                }
            }
        }
        objects
    }

    pub fn read_objects(dirname: &str, obj_names: Vec<&str>) -> BTreeMap<ObjectName, ObjectIn> {
        let mut objects = BTreeMap::new();
        for obj_name in obj_names {
            let path = PathBuf::from(dirname).join(PathBuf::from(obj_name));
            let file_contents = fs::read_to_string(&path).unwrap();
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
            println!("reading {}", file_name.as_str());
            match parse_object_file(file_contents) {
                Ok(object) => {
                    objects.insert(file_name, object);
                }
                Err(err) => panic!("read_objects_from_dir: {err:?}"),
            }
        }
        objects
    }

    pub fn read_lib(dir: &str) -> Result<StaticLib, LibError> {
        StaticLib::parse(dir)
    }
}
