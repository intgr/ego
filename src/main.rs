#[macro_use]
extern crate simple_error;
use acl_sys::*;
use core::mem;
use libc::ssize_t;
use libc::types::common::c95::c_void;
use simple_error::SimpleError;
use std::env;
use std::env::VarError;
use std::ffi::{CStr, CString};
use std::io::Error;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::slice::from_raw_parts;
use std::str::from_utf8;
use users::{get_current_uid, get_current_username, get_user_by_name, uid_t};

struct EgoContext {
    cur_user: String,
    cur_uid: uid_t,
    target_user: String,
    target_uid: uid_t,
}

fn main() {
    let username = "ego";
    let ctx = create_context(username);
    println!(
        "Setting up Alter Ego for user {} ({})",
        ctx.target_user, ctx.target_uid
    );

    let ret = setup_wayland(&ctx);
    if ret.is_err() {
        println!("Error with Wayland: {}", ret.unwrap_err())
    }
}

fn create_context(username: &str) -> EgoContext {
    let cur_user = get_current_username()
        .expect("Unable to resolve current user")
        .into_string()
        .expect("Invalid current user username");
    let user = get_user_by_name(&username).expect("Unable to resolve target user");
    EgoContext {
        cur_user: cur_user,
        cur_uid: get_current_uid(),
        target_user: username.to_string(),
        target_uid: user.uid(),
    }
}

fn check_return(ret: i32, func: &str) {
    if ret != 0 {
        panic!("Error in {}: {}", func, Error::last_os_error());
    }
}

fn add_file_acl(path: &Path, uid: u32, flags: u32) -> Result<(), SimpleError> {
    let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
    unsafe {
        // The POSIX ACL API is just horrible :(
        //        let mut acl: acl_t = acl_init(2);
        let mut acl: acl_t = acl_get_file(c_path.as_ptr(), ACL_TYPE_ACCESS);
        if acl.is_null() {
            bail!("Error reading file ACL: {}", Error::last_os_error());
        }

        let mut entry: acl_entry_t = mem::zeroed();
        check_return(acl_create_entry(&mut acl, &mut entry), "acl_create_entry");

        let mut perm: acl_permset_t = mem::zeroed();
        check_return(acl_get_permset(entry, &mut perm), "acl_get_permset");
        check_return(acl_add_perm(perm, flags), "acl_add_perm");
        check_return(acl_set_permset(entry, perm), "acl_set_permset");
        check_return(acl_set_tag_type(entry, ACL_USER), "acl_set_tag_type");
        check_return(
            acl_set_qualifier(entry, &uid as *const u32 as *const c_void),
            "acl_set_qualifier",
        );

        check_return(acl_calc_mask(&mut acl), "acl_calc_mask");
        let ret = acl_valid(acl);
        if ret != 0 {
            let mut len: ssize_t = 0;
            let txt = acl_to_text(acl, &mut len);
            println!(
                "INVALID ACL: {}",
                from_utf8(from_raw_parts(txt as *const u8, len as usize))
                    .unwrap()
                    .replace('\n', ",")
            );
            bail!("Produced invalid ACL for {}", path.display());
        }
        let ret = acl_set_file(c_path.as_ptr(), ACL_TYPE_ACCESS, acl);
        if ret != 0 {
            bail!(
                "Error writing {} ACL: {}",
                path.display(),
                Error::last_os_error()
            );
        }
        check_return(acl_free(acl), "acl_free");
    }
    Ok(())
}

fn get_wayland_socket(ctx: &EgoContext) -> Result<PathBuf, VarError> {
    let path = env::var("WAYLAND_DISPLAY")?;
    // May be full path or relative
    if path.starts_with('/') {
        Ok(path.into())
    } else {
        Ok(format!("/run/user/{}/{}", ctx.cur_uid, path).into())
    }
}

fn setup_wayland(ctx: &EgoContext) -> Result<(), SimpleError> {
    let path = get_wayland_socket(ctx);
    if path.is_err() {
        println!("Cannot detect Wayland socket: {}", path.err().unwrap());
        return Ok(());
    }
    let path = path.unwrap();
    // /run/user/X has execute perm
    add_file_acl(
        path.as_path().parent().unwrap(),
        ctx.target_uid,
        ACL_EXECUTE,
    )?;
    // socket has rwx perm
    add_file_acl(
        path.as_path(),
        ctx.target_uid,
        ACL_READ | ACL_WRITE | ACL_EXECUTE,
    )?;
    println!("Wayland socket {} configured", path.display());
    Ok(())
}
