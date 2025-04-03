use std::env;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use clap_complete::shells::{Bash, Fish, Zsh};
use clap_complete::Generator;
use log::{info, Level};
use snapbox::Assert;
use snapbox::{file, Data};

use crate::cli::{build_cli, parse_args, Method};
use crate::util::have_command;
use crate::x11::x11_add_acl;
use crate::{check_user_homedir, get_wayland_socket, EgoContext};

/// `vec![]` constructor that converts arguments to String
macro_rules! string_vec {
    ($($x:expr),*) => (vec![$($x.to_string()),*] as Vec<String>);
}

fn snapshot() -> &'static Assert {
    static SNAPSHOT: OnceLock<Assert> = OnceLock::new();
    SNAPSHOT.get_or_init(|| Assert::new().action_env("SNAPSHOTS"))
}

/// Compare log output with snapshot file. Call `testing_logger::setup()` at beginning of test.
fn assert_log_snapshot(expected_path: &Data) {
    testing_logger::validate(|logs| {
        let output = logs.iter().fold(String::new(), |mut a, b| {
            writeln!(a, "{}: {}", b.level.as_str(), b.body).unwrap();
            a
        });
        snapshot().eq(output, expected_path);
    });
}

fn render_completion(generator: impl Generator) -> Data {
    let mut buf = Vec::<u8>::new();
    let mut app = build_cli();
    clap_complete::generate(generator, &mut app, "ego", &mut buf);
    buf.into()
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
    snapshot().eq(
        render_completion(Zsh),
        file!["../varia/ego-completion.zsh"].raw(),
    );
}

/// Run `SNAPSHOTS=overwrite cargo test` to update
#[test]
fn shell_completion_bash() {
    snapshot().eq(
        render_completion(Bash),
        file!["../varia/ego-completion.bash"],
    );
}

/// Run `SNAPSHOTS=overwrite cargo test` to update
#[test]
fn shell_completion_fish() {
    snapshot().eq(
        render_completion(Fish),
        file!["../varia/ego-completion.fish"].raw(),
    );
}

fn test_context() -> EgoContext {
    EgoContext {
        runtime_dir: "/run/user/1000".into(),
        target_user: "ego".into(),
        target_uid: 155,
        target_user_shell: "/bin/bash".into(),
        target_user_homedir: "/home/ego".into(),
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
fn test_x11_error() {
    env::remove_var("DISPLAY");

    let err = x11_add_acl("test", "test").unwrap_err();
    assert_eq!(
        err.to_string(),
        "Connection closed, error during parsing display string"
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
    snapshot().eq(
        build_cli().render_help().to_string(),
        file!["snapshots/ego.help"],
    );
}

#[test]
fn test_have_command() {
    assert!(have_command("sh"));
    assert!(!have_command("what-is-this-i-don't-even"));
}

#[test]
fn test_check_user_homedir() {
    let ctx = EgoContext {
        runtime_dir: PathBuf::default(),
        target_user: "root".to_string(),
        target_uid: 0,
        target_user_shell: PathBuf::default(),
        target_user_homedir: "/root".into(),
    };

    // Capture log output from called functions
    testing_logger::setup();

    info!("TEST: Success (no output)");
    check_user_homedir(&ctx);

    info!("TEST: Home does not exist");
    check_user_homedir(&EgoContext {
        target_user: "nope".into(),
        target_user_homedir: "/tmp/path-does-not-exist.example".into(),
        ..ctx.clone()
    });

    info!("TEST: Permission denied");
    check_user_homedir(&EgoContext {
        target_user_homedir: "/root/path-is-not-accessible.example".into(),
        ..ctx.clone()
    });

    info!("TEST: Wrong owner");
    check_user_homedir(&EgoContext {
        target_uid: 1234,
        ..ctx.clone()
    });

    assert_log_snapshot(&file!["snapshots/check_user_homedir.txt"]);
}
