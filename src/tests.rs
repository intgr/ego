use super::*;

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
