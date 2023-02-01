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

    pub fn build_dir(
        &mut self,
        basepath: Option<&str>,
        libname: Option<&str>,
        object_files: Vec<&str>,
    ) -> Result<(), LibError> {
        self.logger.do_log(
            LogLevel::Info,
            &format!("Building static library at {:?}", basepath),
        );
        match StaticLib::build_static_dirlib(object_files, basepath, libname) {
            Err(e) => panic!("{:?}", e),
            Ok(libname) => {
                self.logger.do_log(
                    LogLevel::Info,
                    &format!("Successfully built static library '{}'", libname),
                );
            }
        }
        Ok(())
    }
}
