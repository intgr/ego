//! Logging for command line output.
//! Adapted from `simple_logger` by Sam Clements: <https://github.com/borntyping/rust-simple_logger>

use crate::util::paint;
use anstyle::{AnsiColor, Color, Style};
use log::{Level, Log, Metadata, Record, trace};

struct SimpleLogger {
    level: Level,
}

const COLOR_TRACE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)));
const COLOR_WARN: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .bold();
const COLOR_ERROR: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Red)))
    .bold();

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
                println!("[{}] {}", paint(COLOR_TRACE, target), record.args());
            }
            Level::Warn => {
                println!("{}: {}", paint(COLOR_WARN, "warning"), record.args());
            }
            Level::Error => {
                println!("{}: {}", paint(COLOR_ERROR, "error"), record.args());
            }
            _ => {
                println!("{}", record.args());
            }
        }
    }

    fn flush(&self) {}
}

/// Initializes the global logger with a `SimpleLogger` instance with
/// `max_log_level` set to a specific log level.
pub fn init_with_level(level: Level) {
    let logger = SimpleLogger { level };
    log::set_boxed_logger(Box::new(logger)).expect("Set logger failed");
    log::set_max_level(level.to_level_filter());

    trace!("Log level {level}");
}
