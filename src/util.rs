use crate::{ErrorWithHint};
use log::debug;
use std::io::ErrorKind;
use std::os::unix::prelude::CommandExt;
use std::path::Path;
use std::process::{Command, Output};
use std::{env, io};

/// Detect if system was booted with systemd init system. Same logic as sd_booted() in libsystemd.
/// https://www.freedesktop.org/software/systemd/man/sd_booted.html
pub fn sd_booted() -> bool {
    Path::new("/run/systemd/system").exists()
}

/// Test if a command is present in $PATH
/// Adapted from https://stackoverflow.com/a/37499032/177663
pub fn have_command<P: AsRef<Path>>(exe_name: P) -> bool {
    env::var_os("PATH").map_or(false, |paths| {
        env::split_paths(&paths).any(|dir| dir.join(&exe_name).is_file())
    })
}

fn report_command_error(err: io::Error, program: &str, args: &[String]) -> ErrorWithHint {
    ErrorWithHint::new(
        format!("Failed to run {}: {}", program, err),
        if err.kind() == ErrorKind::NotFound {
            format!("Try installing package that contains command '{}'", program)
        } else {
            format!("Complete command: {} {}", program, shell_words::join(args))
        },
    )
}

/// Exec command (ending the current process) or return error.
pub fn exec_command(program: &str, args: &[String]) -> Result<(), ErrorWithHint> {
    debug!("Executing: {} {}", program, shell_words::join(args));
    // If this call returns at all, it was an error
    let err: io::Error = Command::new(program).args(args).exec();

    Err(report_command_error(err, program, args))
}

/// Run command as subprocess. Return output if status was 0, otherwise return as error.
pub fn run_command(program: &str, args: &[String]) -> Result<Output, ErrorWithHint> {
    debug!("Running: {} {}", program, shell_words::join(args));
    let ret = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| report_command_error(err, program, args))?;

    if !ret.status.success() {
        return Err(ErrorWithHint::new(
            format!(
                "{} returned {}:\n{}",
                program,
                ret.status.code().unwrap_or(999),
                String::from_utf8_lossy(&ret.stderr).trim()
            ),
            format!("Complete command: {} {}", program, shell_words::join(args)),
        ));
    }
    Ok(ret)
}
