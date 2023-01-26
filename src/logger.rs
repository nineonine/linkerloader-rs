use colored::Colorize;

pub struct Logger {
    logger_ty: LoggerType,
    log_entries: Vec<(LogLevel, String)>,
    pub silent: bool
}

#[derive(Eq, PartialEq)]
enum LoggerType {
    StdOut, TestLogger
}

#[allow(dead_code)]
pub enum LogLevel {
    Info, Warn, Debug, Error
}

impl Logger {
    pub fn new_stdout_logger(silent: bool) -> Logger {
        Logger {
            logger_ty: LoggerType::StdOut,
            log_entries: vec![],
            silent,
        }
    }

    #[allow(dead_code)]
    pub fn new_test_logger(silent: bool) -> Logger {
        Logger {
            logger_ty: LoggerType::TestLogger,
            log_entries: vec![],
            silent,
        }
    }

    fn push(&mut self, lvl: LogLevel, msg: &str) {
        self.log_entries.push((lvl, String::from(msg)));
    }

    pub fn do_log(&mut self, lvl: LogLevel, msg: &str) {
        let pref = match lvl {
            LogLevel::Info => format!("[INFO]").bold(),
            LogLevel::Debug => format!("[DEBUG]").dimmed(),
            LogLevel::Warn => format!("[WARN]").yellow(),
            LogLevel::Error => format!("[ERROR]").red(),
        };
        println!{"{}: {}", pref, msg};
        if self.logger_ty == LoggerType::TestLogger {
            self.push(lvl, msg);
        }
    }

    #[allow(dead_code)]
    pub fn debug(&mut self, msg: &str) {
        self.do_log(LogLevel::Debug, msg);
    }

}
