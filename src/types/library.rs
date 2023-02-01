use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::types::errors::LibError;
use crate::types::object::{parse_object_file, ObjectIn, MAGIC_NUMBER};
use crate::types::symbol_table::SymbolName;
use crate::utils::read_object_file;

type LibObjName = String;
type ModOffset = usize;

#[derive(Debug)]
pub enum StaticLib {
    DirLib {
        symbols: HashMap<LibObjName, HashSet<SymbolName>>,
        objects: HashMap<LibObjName, ObjectIn>,
    },
    FileLib {
        symbols: HashMap<SymbolName, ModOffset>,
        objects: Vec<ObjectIn>,
    },
}

enum LibFormat {
    DirFormat,
    FileFormat,
}

const MAP_FILE_NAME: &str = "MAP";
// const MAGIC_NUMBER_LIB: &str = "LIBRARY";

impl StaticLib {
    pub fn parse(path: &str) -> Result<StaticLib, LibError> {
        match StaticLib::infer_lib_format(path) {
            LibFormat::DirFormat => StaticLib::parse_dir_lib(path),
            LibFormat::FileFormat => StaticLib::parse_file_lib(path),
        }
    }

    fn infer_lib_format(path: &str) -> LibFormat {
        let p = Path::new(path);
        if p.is_dir() {
            LibFormat::DirFormat
        } else {
            LibFormat::FileFormat
        }
    }

    fn parse_dir_lib(path: &str) -> Result<Self, LibError> {
        let mut symbols = HashMap::new();
        let mut objects = HashMap::new();

        let lib_path = Path::new(path);
        let entries = fs::read_dir(lib_path)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();
        for entry in entries {
            let path = entry.path();
            if path.is_file() {
                let file_contents = fs::read_to_string(&path).unwrap();
                let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                if path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .eq(MAP_FILE_NAME)
                {
                    println!("reading MAP file");
                    for l in file_contents.lines() {
                        let toks: Vec<&str> = l.split(' ').map(|s| s.trim()).collect();
                        match toks.as_slice() {
                            [mod_name, syms @ ..] => {
                                let mod_symbols = syms.iter().map(|s| s.to_string()).collect();
                                symbols.insert(mod_name.to_string(), mod_symbols);
                            }
                            _ => panic!("parse_dir_lib: empty MAP entry"),
                        }
                    }
                } else {
                    println!("reading {}", file_name.as_str());
                    match parse_object_file(file_contents) {
                        Ok(object) => {
                            objects.insert(file_name, object);
                        }
                        Err(err) => {
                            return Err(LibError::ObjectParseFailure(err));
                        }
                    }
                }
            }
        }
        Ok(StaticLib::DirLib { symbols, objects })
    }

    fn parse_file_lib(path: &str) -> Result<Self, LibError> {
        let mut objects = vec![];
        let mut symbols = HashMap::new();
        let file_contents = read_object_file(path);
        let file_lines: Vec<&str> = file_contents.lines().collect();
        let hdr: Vec<&str> = file_lines[0].split(' ').map(|s| s.trim()).collect();
        let (num_of_mods, lib_dir_offset) = match hdr.as_slice() {
            ["LIBRARY", num_of_mods, lib_dir_offs] => (
                usize::from_str_radix(num_of_mods, 16).unwrap(),
                usize::from_str_radix(lib_dir_offs, 16).unwrap(),
            ),
            _ => return Err(LibError::ParseLibError),
        };
        for i in 0..num_of_mods {
            let mod_entry: Vec<&str> = file_lines[lib_dir_offset + i - 1]
                .split(' ')
                .map(|s| s.trim())
                .collect();
            match mod_entry.as_slice() {
                [offs, mod_len, syms @ ..] => {
                    let mut obj_in = vec![MAGIC_NUMBER];
                    let offset = usize::from_str_radix(offs, 16).unwrap() - 1;
                    let len = usize::from_str_radix(mod_len, 16).unwrap();
                    #[allow(clippy::needless_range_loop)]
                    for j in offset..offset + len {
                        obj_in.push(file_lines[j]);
                    }
                    let obj_str = obj_in.join("\n");
                    match parse_object_file(obj_str) {
                        Err(e) => {
                            return Err(LibError::ObjectParseFailure(e));
                        }
                        Ok(o) => objects.push(o),
                    };
                    for sym in syms {
                        symbols.insert(sym.to_string(), i);
                    }
                }
                _ => return Err(LibError::ParseLibError),
            };
        }

        Ok(StaticLib::FileLib { symbols, objects })
    }

    fn make_map_file(objects: HashMap<&str, ObjectIn>) -> String {
        let mut map_file = vec![];
        for (name, o) in objects.iter() {
            let mut entry = vec![*name];
            for sym in o.symbol_table.iter() {
                if sym.is_defined() {
                    entry.push(&sym.st_name);
                }
            }
            map_file.push(entry.join(" "));
        }
        map_file.join("\n")
    }

    pub fn build_static_dirlib(
        object_files: Vec<&str>,
        basepath: Option<&str>,
        libname: Option<&str>,
    ) -> Result<String, LibError> {
        let path = match basepath {
            Some(p) => PathBuf::from(p),
            None => env::current_dir().unwrap(),
        };
        let name = match libname {
            Some(n) => PathBuf::from(n),
            None => PathBuf::from("staticlib"),
        };
        let lib_path = path.join(&name);
        match std::fs::create_dir(&lib_path) {
            Ok(_) => (),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!("Error creating static lib dir: {}", e);
                } else {
                    panic!(
                        "static lib at {:?} already exists, deal with it first!",
                        basepath
                    );
                }
            }
        }

        let mut objects = HashMap::new();
        for object_file in object_files.into_iter() {
            let obj_path = path.clone().join(object_file);
            let contents = fs::read_to_string(obj_path)?;
            match parse_object_file(contents) {
                Err(e) => {
                    return Err(LibError::ObjectParseFailure(e));
                }
                Ok(o) => {
                    objects.insert(object_file, o);
                    std::fs::copy(path.join(object_file), lib_path.join(object_file))?;
                }
            }
        }

        let mut map_file = File::create(lib_path.join(MAP_FILE_NAME))?;
        map_file.write_all(StaticLib::make_map_file(objects).as_bytes())?;
        Ok(name.to_str().unwrap().to_owned())
    }
}
