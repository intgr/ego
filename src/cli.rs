use clap::{App, AppSettings, Arg, ArgGroup, ValueHint};
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

pub fn build_cli() -> App<'static> {
    App::new("Alter Ego: run desktop applications under a different local user")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::new("user")
                .short('u')
                .long("user")
                .about("Specify a username (default: ego)")
                .value_name("USER")
                .takes_value(true)
                .value_hint(ValueHint::Username),
        )
        .arg(
            Arg::new("sudo")
                .long("sudo")
                .about("Use 'sudo' to change user (default)"),
        )
        .arg(
            Arg::new("machinectl")
                .long("machinectl")
                .about("Use 'machinectl' to change user"),
        )
        .arg(
            Arg::new("machinectl-bare")
                .long("machinectl-bare")
                .about("Use 'machinectl' but skip xdg-desktop-portal setup"),
        )
        .group(ArgGroup::new("method").args(&["sudo", "machinectl", "machinectl-bare"]))
        .arg(
            Arg::new("command")
                .about("Command name and arguments to run (default: user shell)")
                .multiple(true)
                .value_hint(ValueHint::CommandWithArguments),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .about("Verbose output. Use multiple times for more output.")
                .multiple_occurrences(true),
        )
}

pub fn parse_args<T: Into<OsString> + Clone>(args: impl IntoIterator<Item = T>) -> Args {
    let matches = build_cli().get_matches_from(args);

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
