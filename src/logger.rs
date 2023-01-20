use colored::Colorize;

pub struct Logger {
    logger_ty: LoggerType
  , log_entries: Vec<(LogLevel, String)>
}

#[derive(Eq, PartialEq)]
enum LoggerType {
    StdOut, TestLogger
}

pub enum LogLevel {
    Info, Warn, Error
}

impl Logger {
    pub fn new_stdout_logger() -> Logger {
        Logger {
            logger_ty: LoggerType::StdOut,
            log_entries: vec![],
        }
    }

    pub fn new_test_logger() -> Logger {
        Logger {
            logger_ty: LoggerType::TestLogger,
            log_entries: vec![],
        }
    }

    fn push(&mut self, lvl: LogLevel, msg: &str) {
        self.log_entries.push((lvl, String::from(msg)));
    }

    pub fn do_log(&mut self, lvl: LogLevel, msg: &str) {
        let pref = match lvl {
            LogLevel::Info => format!("[INFO]").bold(),
            LogLevel::Warn => format!("[WARN]").yellow(),
            LogLevel::Error => format!("[ERROR]").red(),
        };
        println!{"{}: {}", pref, msg};
        if self.logger_ty == LoggerType::TestLogger {
            self.push(lvl, msg);
        }
}


}
