use crate::{
    logger::*,
    types::{errors::LibError, library::StaticLib},
};

pub struct Librarian {
    logger: Logger,
}

impl Librarian {
    pub fn new(silent: bool) -> Self {
        Librarian {
            logger: Logger::new_stdout_logger(silent),
        }
    }

    pub fn build_libdir(
        &mut self,
        basepath: Option<&str>,
        libname: Option<&str>,
        object_files: Vec<&str>,
    ) -> Result<(), LibError> {
        self.logger.do_log(
            LogLevel::Info,
            &format!("Building static libdir at {basepath:?}"),
        );
        match StaticLib::build_static_dirlib(object_files, basepath, libname) {
            Err(e) => panic!("{e:?}"),
            Ok(libname) => {
                self.logger.do_log(
                    LogLevel::Info,
                    &format!("Successfully built static libdir '{libname}'"),
                );
            }
        }
        Ok(())
    }

    pub fn build_libfile(
        &mut self,
        basepath: Option<&str>,
        libname: Option<&str>,
        object_files: Vec<&str>,
    ) -> Result<(), LibError> {
        self.logger.do_log(
            LogLevel::Info,
            &format!("Building static libfile at {basepath:?}"),
        );
        match StaticLib::build_static_filelib(object_files, basepath, libname) {
            Err(e) => panic!("{e:?}"),
            Ok(libname) => {
                self.logger.do_log(
                    LogLevel::Info,
                    &format!("Successfully built static libfile '{libname}'"),
                );
            }
        }
        Ok(())
    }

    pub fn build_static_shared_lib(
        &mut self,
        path: &str,
        libdeps: Vec<String>,
        start: i32,
    ) -> Result<(), LibError> {
        self.logger.do_log(
            LogLevel::Info,
            &format!("Building statically linked shared library at {path:?}"),
        );
        let mut libs = vec![];
        for libname in libdeps.iter() {
            let lib = StaticLib::parse(libname)?;
            libs.push(lib);
        }
        println!("**** num of libs parsed: {}", libs.len());
        match StaticLib::parse(path)?.build_shared_lib(start, libs, path) {
            Err(e) => panic!("{e:?}"),
            Ok(_) => {
                self.logger
                    .do_log(LogLevel::Info, "Successfully built static shared lib");
            }
        }
        Ok(())
    }
}
