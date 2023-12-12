//! Error handling helpers and the `ErrorWithHint` type for more verbose error messages.

use crate::util::paint;
use anstyle::{AnsiColor, Color, Style};
use log::error;
use std::error::Error;
use std::fmt;

/// Shorter alias for `Box<dyn Error>`
pub type AnyErr = Box<dyn Error>;

const COLOR_HINT: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Green)))
    .bold();

/// Advanced error type that can supply hints to the user
#[derive(Debug)]
pub struct ErrorWithHint {
    err: String,
    hint: String,
}

impl ErrorWithHint {
    pub fn new(err: String, hint: String) -> ErrorWithHint {
        ErrorWithHint { err, hint }
    }
}

impl Error for ErrorWithHint {}

impl fmt::Display for ErrorWithHint {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.err.fmt(f)?;

        if !self.hint.is_empty() {
            write!(f, "\n{}: {}", paint(COLOR_HINT, "hint"), self.hint)?;
        }
        Ok(())
    }
}

pub fn print_error(err: &AnyErr) {
    error!("{err}");
}
