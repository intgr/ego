use clap::{Arg, ArgGroup, Command, ValueHint};
use log::Level;
use std::ffi::OsString;

#[derive(Debug, PartialEq, Eq)]
pub enum Method {
    Sudo,
    Machinectl,
    MachinectlBare,
}

/// Data type for parsed settings
pub struct Args {
    pub user: String,
    pub command: Vec<String>,
    pub log_level: Level,
    pub method: Option<Method>,
}

pub fn build_cli() -> Command<'static> {
    Command::new("Alter Ego: run desktop applications under a different local user")
        .trailing_var_arg(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("user")
                .short('u')
                .long("user")
                .value_name("USER")
                .help("Specify a username (default: ego)")
                .takes_value(true)
                .value_hint(ValueHint::Username),
        )
        .arg(
            Arg::new("sudo")
                .long("sudo")
                .help("Use 'sudo' to change user"),
        )
        .arg(
            Arg::new("machinectl")
                .long("machinectl")
                .help("Use 'machinectl' to change user (default, if available)"),
        )
        .arg(
            Arg::new("machinectl-bare")
                .long("machinectl-bare")
                .help("Use 'machinectl' but skip xdg-desktop-portal setup"),
        )
        .group(ArgGroup::new("method").args(&["sudo", "machinectl", "machinectl-bare"]))
        .arg(
            Arg::new("command")
                .help("Command name and arguments to run (default: user shell)")
                .multiple_values(true)
                .value_hint(ValueHint::CommandWithArguments),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("Verbose output. Use multiple times for more output."),
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
            Some(Method::Machinectl)
        } else if matches.is_present("machinectl-bare") {
            Some(Method::MachinectlBare)
        } else if matches.is_present("sudo") {
            Some(Method::Sudo)
        } else {
            None
        },
    }
}
