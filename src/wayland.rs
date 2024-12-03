use crate::errors::AnyErr;
use log::{info, warn};
use nix::libc::stat;
use nix::unistd::pipe;
use std::fs::remove_file;
use std::mem;
use std::os::fd::AsFd;
use std::os::unix::net::UnixListener;
use std::path::Path;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::security_context::v1::client::wp_security_context_manager_v1::WpSecurityContextManagerV1;
use wayland_protocols::wp::security_context::v1::client::wp_security_context_v1::WpSecurityContextV1;
use wayland_protocols::wp::security_context::v1::client::{
    wp_security_context_manager_v1, wp_security_context_v1,
};

pub struct NoopState {}

impl Dispatch<WlRegistry, GlobalListContents> for NoopState {
    fn event(
        _: &mut NoopState,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<NoopState>,
    ) {
    }
}

impl Dispatch<WpSecurityContextManagerV1, (), NoopState> for NoopState {
    fn event(
        _: &mut NoopState,
        _: &WpSecurityContextManagerV1,
        _: wp_security_context_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<NoopState>,
    ) {
    }
}

impl Dispatch<WpSecurityContextV1, ()> for NoopState {
    fn event(
        _: &mut NoopState,
        _: &WpSecurityContextV1,
        _: wp_security_context_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<NoopState>,
    ) {
    }
}

pub fn wayland_security_context() -> Result<(), AnyErr> {
    let sandbox_socket_path: &Path = Path::new("/tmp/wayland-sandbox.sock");

    if sandbox_socket_path.exists() {
        info!(
            "Removing previous socket path {}",
            sandbox_socket_path.display()
        );
        remove_file(sandbox_socket_path)?;
    }

    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<NoopState>(&conn)?;
    let qh: QueueHandle<NoopState> = queue.handle();

    // https://docs.rs/wayland-protocols/latest/wayland_protocols/wp/security_context/v1/client/wp_security_context_manager_v1/struct.WpSecurityContextManagerV1.html
    let sec_ctx_manager = match globals.bind::<WpSecurityContextManagerV1, _, _>(&qh, 1..=1, ()) {
        Ok(manager) => manager,
        Err(e) => {
            warn!("Skipping Wayland security context: {}", e);
            return Ok(());
        }
    };

    let listen_socket = map_err_with!(
        UnixListener::bind(sandbox_socket_path),
        "Cannot listen on socket {}",
        sandbox_socket_path.display()
    )?;
    let listen_fd = listen_socket.as_fd();

    let (close_pipe, remote_pipe) = pipe()?;

    let context = sec_ctx_manager.create_listener(listen_fd, remote_pipe.as_fd(), &qh, ());
    context.set_sandbox_engine("org.juffo.ego".into());
    // FIXME!
    context.set_app_id("asd".into());
    context.set_instance_id("asd".into());
    context.commit();
    conn.roundtrip()?;

    drop(listen_socket);

    // Leak the "read" side of close_fd, to keep it alive for the lifetime of the process.
    // TODO: Persist it in EgoContext or other place instead?
    mem::forget(close_pipe);

    info!(
        "Wayland security context created, socket: {}",
        sandbox_socket_path.display()
    );

    Ok(())
}
