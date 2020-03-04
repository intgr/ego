//! Error handling helpers and the `ErrorWithHint` type for more verbose error messages.

use ansi_term::Colour::Green;
use log::error;
use std::error::Error;
use std::fmt;

/// Shorter alias for `Box<dyn Error>`
pub type AnyErr = Box<dyn Error>;

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
        self.err.fmt(f)
    }
}

pub fn print_error(err: AnyErr) {
    // Look ma', dynamic typing in Rust!
    if let Some(errhint) = err.downcast_ref::<ErrorWithHint>() {
        error!("{}\n{}: {}", errhint.err, Green.paint("hint"), errhint.hint);
    } else {
        error!("{}", err);
    }
}
