//! Logging for command line output.
//! Adapted from simple_logger by Sam Clements: https://github.com/borntyping/rust-simple_logger

extern crate log;

use ansi_term::Colour::{Purple, Red, Yellow};
use log::{trace, Level, Log, Metadata, Record};

struct SimpleLogger {
    level: Level,
}

impl Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.level() {
            Level::Trace => {
                let target = if record.target().is_empty() {
                    record.module_path().unwrap_or_default()
                } else {
                    record.target()
                };
                println!("[{}] {}", Purple.paint(target), record.args());
            }
            Level::Warn => {
                println!("{}: {}", Yellow.paint("warning"), record.args());
            }
            Level::Error => {
                println!("{}: {}", Red.paint("error"), record.args());
            }
            _ => {
                println!("{}", record.args());
            }
        }
    }

    fn flush(&self) {}
}

/// Initializes the global logger with a SimpleLogger instance with
/// `max_log_level` set to a specific log level.
pub fn init_with_level(level: Level) {
    let logger = SimpleLogger { level };
    log::set_boxed_logger(Box::new(logger)).expect("Set logger failed");
    log::set_max_level(level.to_level_filter());

    trace!("Log level {}", level);
}
