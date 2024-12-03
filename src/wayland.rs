use crate::errors::AnyErr;
use log::{info, warn};
use nix::unistd::pipe;
use std::fs::remove_file;
use std::mem::forget;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixListener;
use std::path::Path;
use wayrs_client::global::GlobalsExt;
use wayrs_client::Connection;
use wayrs_client::IoMode::Blocking;
use wayrs_protocols::security_context_v1::WpSecurityContextManagerV1;

pub fn wayland_security_context() -> Result<(), AnyErr> {
    let sandbox_socket_path: &Path = Path::new("/tmp/wayland-sandbox.sock");

    let (mut conn, globals) = Connection::<()>::connect_and_collect_globals()?;

    if sandbox_socket_path.exists() {
        info!(
            "Removing previous socket path {}",
            sandbox_socket_path.display()
        );
        remove_file(sandbox_socket_path)?;
    }

    let listen_socket = map_err_with!(
        UnixListener::bind(sandbox_socket_path),
        "Cannot listen on socket {}",
        sandbox_socket_path.display()
    )?;
    // listen_socket
    let listen_fd = OwnedFd::from(listen_socket);
    let (local_close_fd, remote_close_fd) = pipe()?;

    // https://docs.rs/wayrs-protocols/latest/wayrs_protocols/security_context_v1/wp_security_context_manager_v1/struct.WpSecurityContextManagerV1.html
    let sec_ctx_manager = match globals.bind::<WpSecurityContextManagerV1, _>(&mut conn, 1..=1) {
        Ok(manager) => manager,
        Err(e) => {
            warn!("Skipping Wayland security context: {}", e);
            return Ok(());
        }
    };

    let context = sec_ctx_manager.create_listener(&mut conn, listen_fd, remote_close_fd);

    // TODO MSRV? 1.77+
    context.set_sandbox_engine(&mut conn, c"org.juffo.ego".into());
    // context.set_app_id(&mut conn, c"...".into());
    // context.set_instance_id(&mut conn, c"...".into());
    context.commit(&mut conn);
    conn.flush(Blocking)?;

    // Leak the "read" side of close_fd, to keep it alive for the lifetime of the process.
    // TODO: Persist it in EgoContext or other place instead?
    forget(local_close_fd);

    info!(
        "Wayland security context created, socket: {}",
        sandbox_socket_path.display()
    );

    Ok(())
}
