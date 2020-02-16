use super::*;

/// `vec![]` constructor that converts arguments to String
macro_rules! string_vec {
    ($($x:expr),*) => (vec![$($x.to_string()),*] as Vec<String>);
}

fn test_context() -> EgoContext {
    EgoContext {
        runtime_dir: "/run/user/1000".into(),
        target_user: "ego".into(),
        target_uid: 155,
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
}
