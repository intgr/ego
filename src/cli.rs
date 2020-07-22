use clap::{App, AppSettings, Arg, ArgGroup};
use log::Level;
use std::ffi::OsString;

#[derive(Debug, PartialEq)]
pub enum Method {
    Sudo,
    Machinectl,
    MachinectlBare,
}

/// Data type for parsed settings
pub struct Args {
    pub user: String,
    pub command: Vec<String>,
    pub log_level: log::Level,
    pub method: Method,
}

pub fn parse_args<T: Into<OsString> + Clone>(args: impl IntoIterator<Item = T>) -> Args {
    let matches = App::new("Alter Ego: run desktop applications under a different local user")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("user")
                .short("u")
                .long("user")
                .value_name("USER")
                .help("Specify a username (default: ego)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sudo")
                .long("sudo")
                .help("Use 'sudo' to change user (default)"),
        )
        .arg(
            Arg::with_name("machinectl")
                .long("machinectl")
                .help("Use 'machinectl' to change user"),
        )
        .arg(
            Arg::with_name("machinectl-bare")
                .long("machinectl-bare")
                .help("Use 'machinectl' but skip xdg-desktop-portal setup"),
        )
        .group(ArgGroup::with_name("method").args(&["sudo", "machinectl"]))
        .arg(
            Arg::with_name("command")
                .help("Command name and arguments to run (default: user shell)")
                .multiple(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Verbose output. Use multiple times for more output."),
        )
        .get_matches_from(args);

    Args {
        user: matches.value_of("user").unwrap_or("ego").to_string(),
        command: matches
            .values_of("command")
            .unwrap_or_default()
            .map(|v| v.to_string())
            .collect(),
        log_level: match matches.occurrences_of("verbose") {
            0 => Level::Warn,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
        },
        method: if matches.is_present("machinectl") {
            Method::Machinectl
        } else if matches.is_present("machinectl-bare") {
            Method::MachinectlBare
        } else {
            Method::Sudo
        },
    }
}
