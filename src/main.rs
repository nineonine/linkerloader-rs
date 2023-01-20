mod logger;

use linkerloader::types::object::MAGIC_NUMBER;
use logger::{Logger, LogLevel};

fn main() {
    let mut logger = Logger::new_stdout_logger();
    logger.do_log(LogLevel::Info, "Linker/Loader v0.1");
    logger.do_log(LogLevel::Info, &format!("MAGIC NUMBER: {}", MAGIC_NUMBER));
}
