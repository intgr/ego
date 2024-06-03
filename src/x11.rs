use log::debug;
use xcb::x::{ChangeHosts, Family, HostMode};
use xcb::Connection;

use crate::errors::AnyErr;

pub fn x11_add_acl(type_tag: &str, value: &str) -> Result<(), AnyErr> {
    let (conn, _screen_num) = Connection::connect(None)?;

    debug!("X11: Adding XHost entry SI:{type_tag}:{value}");

    let result = conn.send_and_check_request(&ChangeHosts {
        mode: HostMode::Insert,
        family: Family::ServerInterpreted,
        address: format!("{type_tag}\x00{value}").as_bytes(),
    });
    map_err_with!(result, "Error adding XHost entry")?;

    Ok(())
}
