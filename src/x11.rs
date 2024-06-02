use simple_error::SimpleError;
use std::mem;
use std::os::raw::{c_char, c_int};
use std::ptr::null_mut;

use x11::xlib;
use x11::xlib::{FamilyServerInterpreted, XAddHost, XCloseDisplay, XHostAddress, XOpenDisplay};

/// Based on code by Vadzim Dambrouski from:
/// <https://github.com/pftbest/x11-rust-example/blob/master/src/lib.rs>
pub struct Display {
    raw: *mut xlib::Display,
}

impl Display {
    pub fn open() -> Result<Self, SimpleError> {
        let display = unsafe { XOpenDisplay(null_mut()) };
        if display.is_null() {
            bail!("Could not open X11 display");
        }
        Ok(Display { raw: display })
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe { XCloseDisplay(self.raw) };
    }
}

/// <https://www.x.org/releases/X11R7.5/doc/man/man3/XAddHost.3.html#sect4>
/// Submitted to upstream: <https://github.com/AltF02/x11-rs/pull/152>
#[repr(C)]
pub struct XServerInterpretedAddress {
    pub typelength: c_int,
    pub valuelength: c_int,
    pub type_: *mut c_char,
    pub value: *mut c_char,
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_possible_wrap)]
pub fn new_siaddr(type_: &str, value: &str) -> XServerInterpretedAddress {
    XServerInterpretedAddress {
        typelength: type_.len() as c_int,
        valuelength: value.len() as c_int,
        type_: type_.as_ptr() as *mut c_char,
        value: value.as_ptr() as *mut c_char,
    }
}

#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::ptr_as_ptr)]
pub fn x11_add_acl(type_: &str, value: &str) -> Result<(), SimpleError> {
    let display = Display::open()?;

    // Construct message
    let mut siaddr = new_siaddr(type_, value);
    let mut acl = XHostAddress {
        family: FamilyServerInterpreted,
        address: std::ptr::addr_of_mut!(siaddr) as *mut c_char,
        length: mem::size_of::<XServerInterpretedAddress>() as c_int,
    };
    // Doc: https://www.x.org/releases/X11R7.5/doc/man/man3/XAddHost.3.html
    let ret = unsafe { XAddHost(display.raw, &mut acl) };
    // According to xhost code, return 1 is success
    if ret != 1 {
        bail!("XAddHost returned {}", ret);
    }
    Ok(())
}
