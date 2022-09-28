use clap::{Arg, ArgAction, ArgGroup, Command, ValueHint};
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

pub fn build_cli() -> Command {
    Command::new("ego")
        .about("Alter Ego: run desktop applications under a different local user")
        .disable_version_flag(true)
        .arg(
            Arg::new("user")
                .short('u')
                .long("user")
                .value_name("USER")
                .default_value("ego")
                .help("Specify a username (default: ego)")
                .value_hint(ValueHint::Username),
        )
        .arg(
            Arg::new("sudo")
                .long("sudo")
                .action(ArgAction::SetTrue)
                .help("Use 'sudo' to change user"),
        )
        .arg(
            Arg::new("machinectl")
                .long("machinectl")
                .action(ArgAction::SetTrue)
                .help("Use 'machinectl' to change user (default, if available)"),
        )
        .arg(
            Arg::new("machinectl-bare")
                .long("machinectl-bare")
                .action(ArgAction::SetTrue)
                .help("Use 'machinectl' but skip xdg-desktop-portal setup"),
        )
        .group(ArgGroup::new("method").args(&["sudo", "machinectl", "machinectl-bare"]))
        .arg(
            Arg::new("command")
                .help("Command name and arguments to run (default: user shell)")
                .num_args(1..)
                .trailing_var_arg(true)
                .value_hint(ValueHint::CommandWithArguments),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help("Verbose output. Use multiple times for more output."),
        )
}

pub fn parse_args<T: Into<OsString> + Clone>(args: impl IntoIterator<Item = T>) -> Args {
    let matches = build_cli().get_matches_from(args);

    Args {
        user: matches.get_one::<String>("user").unwrap().clone(),
        command: matches
            .get_many("command")
            .unwrap_or_default()
            .cloned()
            .collect(),
        log_level: match matches.get_count("verbose") {
            0 => Level::Warn,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
        },
        method: if matches.get_flag("machinectl") {
            Some(Method::Machinectl)
        } else if matches.get_flag("machinectl-bare") {
            Some(Method::MachinectlBare)
        } else if matches.get_flag("sudo") {
            Some(Method::Sudo)
        } else {
            None
        },
    }
}
