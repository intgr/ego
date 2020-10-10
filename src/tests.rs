use crate::cli::{build_cli, parse_args, Method};
use crate::{get_wayland_socket, EgoContext};
use ansi_term::Colour::{Cyan, Red};
use clap_generate::generators::{Bash, Fish, Zsh};
use clap_generate::Generator;
use log::Level;
use std::env;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;

/// `vec![]` constructor that converts arguments to String
macro_rules! string_vec {
    ($($x:expr),*) => (vec![$($x.to_string()),*] as Vec<String>);
}

/// Helper for snapshot testing, we abuse it to keep shell completions up to date.
fn snapshot_match(path: &str, data: &[u8]) {
    if cfg!(feature = "update-snapshots") {
        let mut file = File::create(path).unwrap();
        file.write(data.as_ref()).unwrap();
        file.flush().unwrap();
    } else {
        let mut snapshot_data = Vec::<u8>::new();
        match File::open(path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => panic!("{}: {}", path, e),
            Ok(mut file) => {
                file.read_to_end(&mut snapshot_data).unwrap();
            }
        }
        if data != snapshot_data {
            panic!(
                "\n{}\n{}\n",
                Red.paint(format!("Snapshot {} out of date", path)),
                Cyan.paint("HINT: run 'cargo test --features=update-snapshots' to update")
            );
        }
    }
}

fn render_completion<G: Generator>() -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    let mut app = build_cli();
    clap_generate::generate::<G, _>(&mut app, "ego", &mut buf);
    buf.write(b"\n").unwrap();

    buf
}

/// Unit tests may seem like a weird place to update shell completion files, but this is like
/// snapshot testing, which guarantees the file is never out of date.
///
/// Also we don't have to lug around `clap_generate` code in the `ego` binary itself.
///
/// run 'cargo test --features=update-snapshots' to update
///
/// Usage with zsh:
/// ```
/// cp varia/ego-completion.zsh /usr/local/share/zsh/site-functions/_ego
/// ```
#[test]
fn shell_completions() {
    snapshot_match("varia/ego-completion.zsh", &render_completion::<Zsh>());
    snapshot_match("varia/ego-completion.bash", &render_completion::<Bash>());
    snapshot_match("varia/ego-completion.fish", &render_completion::<Fish>());
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
fn test_parse_args() {
    // Empty command line (defaults)
    let args = parse_args(vec!["ego"]);
    assert_eq!(args.user, "ego".to_string());
    assert_eq!(args.command, string_vec![]);
    assert_eq!(args.log_level, Level::Warn);
    assert_eq!(args.method, Method::Sudo);

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
        Method::Machinectl
    );
}
