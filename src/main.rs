#[macro_use]
extern crate simple_error;

use std::env;
use std::env::VarError;
use std::path::{Path, PathBuf};

use acl_sys::*;
use simple_error::SimpleError;
use users::{get_current_uid, get_current_username, get_user_by_name, uid_t};

use crate::acl::PosixACL;
use crate::acl::Qualifier::User;

mod acl;

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

fn add_file_acl(path: &Path, uid: u32, flags: u32) -> Result<(), SimpleError> {
    let mut acl = PosixACL::read_acl(path)?;
    acl.set(User(uid), flags);
    acl.write_acl(path)?;

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
