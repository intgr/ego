use std::env;
use std::path::Path;

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
