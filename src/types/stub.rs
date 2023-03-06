use crate::common::{
    Address, LibName, StubMemberName, LIB_NAME_FILE, MAP_FILE_NAME, STUB_MAGIC_NUMBER,
};
use crate::types::{
    errors::{LibError, ParseError},
    symbol_table::SymbolName,
};
use either::Either::{self, Left, Right};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct StubMember {
    pub name: StubMemberName,
    pub syms: BTreeMap<SymbolName, Either<Address, LibName>>,
}

impl StubMember {
    pub fn new(name: StubMemberName, syms: BTreeMap<SymbolName, Either<Address, LibName>>) -> Self {
        StubMember { name, syms }
    }

    pub fn serialize(&self) -> String {
        let mut ret = vec![STUB_MAGIC_NUMBER.to_owned()];
        for (symname, addr_or_libname) in self.syms.iter() {
            let v = match addr_or_libname {
                Left(addr) => format!("{addr:X}"),
                Right(libname) => libname.to_owned(),
            };
            ret.push(format!("{symname} {v}"));
        }
        ret.join("\n")
    }
}

#[derive(Debug)]
pub struct StubLib {
    pub libname: LibName,
    pub members: BTreeMap<StubMemberName, StubMember>,
    pub defs: BTreeMap<StubMemberName, Vec<SymbolName>>,
    pub deps: Vec<LibName>,
}

impl StubLib {
    pub fn new(name: String) -> Self {
        StubLib {
            libname: name,
            members: BTreeMap::new(),
            defs: BTreeMap::new(),
            deps: Vec::new(),
        }
    }

    pub fn parse(path: &str) -> Result<Self, LibError> {
        let libpath = Path::new(path);
        let mut libname = String::from("stublib");

        let mut members = BTreeMap::new();
        let mut defs = BTreeMap::new();
        let mut deps = Vec::new();

        let entries = fs::read_dir(libpath)
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
                        let toks: Vec<String> = l.split(' ').map(|s| s.trim().to_owned()).collect();
                        match toks.as_slice() {
                            [mod_name, syms @ ..] => {
                                let mod_symbols = syms
                                    .iter()
                                    .map(|s| SymbolName::SName(s.to_owned()))
                                    .collect();
                                defs.insert(mod_name.to_string(), mod_symbols);
                            }
                            _ => panic!("StubLib::parse: empty MAP entry"),
                        }
                    }
                } else if path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .eq(LIB_NAME_FILE)
                {
                    println!("reading LIBRARY NAME file");
                    let mut input = file_contents.lines();
                    if let Some(l) = input.next() {
                        libname = l.to_owned();
                    }
                    for l in input {
                        deps.push(l.to_owned())
                    }
                } else {
                    println!("reading stub member {}", file_name.as_str());
                    match StubLib::parse_stub_member(&file_name, &file_contents) {
                        Ok(member) => {
                            members.insert(file_name, member);
                        }
                        Err(e) => return Err(LibError::StubMemberParseFailure(e)),
                    }
                }
            }
        }
        Ok(StubLib {
            libname,
            members,
            defs,
            deps,
        })
    }

    fn parse_stub_member(libname: &str, file_contents: &str) -> Result<StubMember, ParseError> {
        let mut input = file_contents.lines();

        match input.next() {
            None => return Err(ParseError::MissingMagicNumber),
            Some(mn) => {
                if mn != STUB_MAGIC_NUMBER {
                    return Err(ParseError::InvalidMagicNumber);
                }
            }
        }

        let mut syms = BTreeMap::new();
        for s in input {
            let vs: Vec<&str> = s.split_ascii_whitespace().collect();
            match vs.as_slice() {
                [symname, v] => {
                    let n = SymbolName::SName(String::from(*symname));
                    match i32::from_str_radix(v, 16) {
                        Err(_) => {
                            // undefined symbol - value is lib name where defined
                            syms.insert(n, Right(String::from(*v)));
                        }
                        Ok(addr) => {
                            // abs address in linked lib object
                            syms.insert(n, Left(addr));
                        }
                    }
                }
                _ => return Err(ParseError::UnexpectedParseError),
            }
        }
        Ok(StubMember::new(libname.to_owned(), syms))
    }

    pub fn write_to_disk(
        &self,
        basepath: Option<&str>,
        libname: Option<&str>,
    ) -> Result<(), LibError> {
        let path = match basepath {
            Some(p) => PathBuf::from(p),
            None => env::current_dir().unwrap(),
        };
        let name = match libname {
            Some(n) => PathBuf::from(n),
            None => PathBuf::from("stublib"),
        };
        let lib_path = path.join(&name);
        match std::fs::create_dir(&lib_path) {
            Ok(_) => (),
            Err(e) => {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    panic!("Error creating stub lib: {e}");
                } else {
                    panic!("stub lib at {basepath:?} already exists, deal with it first!");
                }
            }
        }

        let mut map_file = File::create(lib_path.join(MAP_FILE_NAME))?;
        map_file.write_all(self.make_map_file().as_bytes())?;
        let mut library_name_file = File::create(lib_path.join(LIB_NAME_FILE))?;
        library_name_file.write_all(self.make_library_file().as_bytes())?;
        for (modname, member) in self.members.iter() {
            let mut modfile = File::create(lib_path.join(modname))?;
            modfile.write_all(member.serialize().as_bytes())?;
        }
        Ok(())
    }

    fn make_map_file(&self) -> String {
        let mut map_file = vec![MAP_FILE_NAME.to_owned()];
        for (modname, member) in self.members.iter() {
            let mut entry = vec![modname.to_owned()];
            for (k, sym) in member.syms.iter() {
                if sym.is_left() {
                    entry.push(k.to_string());
                }
            }
            map_file.push(entry.join(" "));
        }
        map_file.join("\n")
    }

    fn make_library_file(&self) -> String {
        let mut ret = vec![];
        ret.push(self.libname.to_owned());
        for dep in self.deps.iter() {
            ret.push(dep.to_owned());
        }
        ret.join("\n")
    }
}
