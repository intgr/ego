use log::{debug, warn};
use xcb::x::{ChangeHosts, Family, HostMode};
use xcb::{ConnError, Connection};

use crate::errors::{AnyErr, ErrorWithHint, print_error};
use crate::util::run_command;

/// Try `libxcb`, fall back to `xhost`.
pub fn x11_add_acl_with_fallback(type_tag: &str, value: &str) -> Result<(), AnyErr> {
    if let Err(err) = x11_xcb_add_acl(type_tag, value) {
        print_error(&err);
        match x11_xhost_add_acl(type_tag, value) {
            Ok(()) => {
                warn!("Successfully fell back to --old-xhost");
                warn!("If you believe this is an error, please report a bug.");
            }
            Err(err) => {
                bail!("Fallback also failed. {err}");
            }
        }
    }
    Ok(())
}

pub fn x11_xcb_add_acl(type_tag: &str, value: &str) -> Result<(), AnyErr> {
    let conn: Connection = match Connection::connect(None) {
        Ok((conn, _screen_num)) => conn,
        Err(ConnError::LibrariesNotLoaded) => {
            return Err(ErrorWithHint::new(
                "libxcb library could not be loaded".into(),
                "Try installing package that contains library 'libxcb.so'".into(),
            )
            .into());
        }
        Err(err) => bail!("Error connecting to X11: {err}"),
    };

    debug!("X11: Adding XHost entry SI:{type_tag}:{value}");

    let result = conn.send_and_check_request(&ChangeHosts {
        mode: HostMode::Insert,
        family: Family::ServerInterpreted,
        address: format!("{type_tag}\x00{value}").as_bytes(),
    });
    map_err_with!(result, "Error adding XHost entry")?;

    Ok(())
}

/// Legacy method
pub fn x11_xhost_add_acl(type_tag: &str, value: &str) -> Result<(), AnyErr> {
    let grant = format!("+si:{type_tag}:{value}");
    run_command("xhost", &[grant])?;
    Ok(())
}
