use std::env;
use std::path::PathBuf;

use clap_complete::shells::{Bash, Fish, Zsh};
use clap_complete::Generator;
use log::Level;

use crate::cli::{build_cli, parse_args, Method};
use crate::util::have_command;
use crate::{get_wayland_socket, EgoContext};

/// `vec![]` constructor that converts arguments to String
macro_rules! string_vec {
    ($($x:expr),*) => (vec![$($x.to_string()),*] as Vec<String>);
}

fn render_completion(generator: impl Generator) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    let mut app = build_cli();
    clap_complete::generate(generator, &mut app, "ego", &mut buf);
    // XXX clap_complete doesn't append newline to zsh completions.
    if !buf.ends_with(b"\n") {
        buf.push(b'\n');
    }

    buf
}

/// Unit tests may seem like a weird place to update shell completion files, but snapshot testing
/// guarantees the files are never out of date.
///
/// Also we don't have to lug around `clap_complete` code in the `ego` binary itself.
///
/// Run `SNAPSHOTS=overwrite cargo test` to update
///
/// Usage with zsh:
/// ```
/// cp varia/ego-completion.zsh /usr/local/share/zsh/site-functions/_ego
/// ```
#[test]
fn shell_completion_zsh() {
    snapbox::assert_eq_path("varia/ego-completion.zsh", render_completion(Zsh));
}

/// Run `SNAPSHOTS=overwrite cargo test` to update
#[test]
fn shell_completion_bash() {
    snapbox::assert_eq_path("varia/ego-completion.bash", render_completion(Bash));
}

/// Run `SNAPSHOTS=overwrite cargo test` to update
#[test]
fn shell_completion_fish() {
    snapbox::assert_eq_path("varia/ego-completion.fish", render_completion(Fish));
}

fn test_context() -> EgoContext {
    EgoContext {
        runtime_dir: "/run/user/1000".into(),
        target_user: "ego".into(),
        target_uid: 155,
        target_user_shell: "/bin/bash".into(),
    }
}

#[test]
fn wayland_socket() {
    let ctx = test_context();
    env::remove_var("WAYLAND_DISPLAY");
    assert_eq!(get_wayland_socket(&ctx).unwrap(), None);

    env::set_var("WAYLAND_DISPLAY", "wayland-7");
    assert_eq!(
        get_wayland_socket(&ctx).unwrap().unwrap(),
        PathBuf::from("/run/user/1000/wayland-7")
    );

    env::set_var("WAYLAND_DISPLAY", "/tmp/wayland-7");
    assert_eq!(
        get_wayland_socket(&ctx).unwrap().unwrap(),
        PathBuf::from("/tmp/wayland-7")
    );
}

#[test]
fn test_cli() {
    build_cli().debug_assert();
}

#[test]
fn test_parse_args() {
    // Empty command line (defaults)
    let args = parse_args(vec!["ego"]);
    assert_eq!(args.user, "ego".to_string());
    assert_eq!(args.command, string_vec![]);
    assert_eq!(args.log_level, Level::Warn);
    assert_eq!(args.method, None);

    // --user
    assert_eq!(
        parse_args(vec!["ego", "-u", "myself"]).user,
        "myself".to_string()
    );
    // command with -flags
    assert_eq!(
        parse_args(vec!["ego", "ls", "-la"]).command,
        string_vec!["ls", "-la"]
    );
    // verbosity
    assert_eq!(parse_args(vec!["ego", "-v"]).log_level, Level::Info);
    assert_eq!(parse_args(vec!["ego", "-v", "-v"]).log_level, Level::Debug);
    assert_eq!(parse_args(vec!["ego", "-vvvvvv"]).log_level, Level::Trace);
    // --machinectl
    assert_eq!(
        parse_args(vec!["ego", "--machinectl"]).method,
        Some(Method::Machinectl)
    );
}

#[test]
fn test_cli_help() {
    snapbox::assert_eq_path(
        "src/snapshots/ego.help",
        build_cli().render_help().to_string(),
    );
}

#[test]
fn test_have_command() {
    assert!(have_command("sh"));
    assert!(!have_command("what-is-this-i-don't-even"));
}
