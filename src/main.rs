#[macro_use]
extern crate simple_error;

use std::env;
use std::env::VarError;
use std::path::{Path, PathBuf};
use std::process::exit;

use posix_acl::Qualifier;
use posix_acl::{PosixACL, ACL_EXECUTE, ACL_READ, ACL_WRITE};
use simple_error::SimpleError;
use users::{get_current_uid, get_current_username, get_user_by_name, uid_t};

struct EgoContext {
    #[allow(dead_code)] // FIXME
    cur_user: String,
    #[allow(dead_code)] // FIXME
    cur_uid: uid_t,
    runtime_dir: PathBuf,
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

    let ret = prepare_runtime_dir(&ctx);
    if ret.is_err() {
        println!("Error setting up XDG_RUNTIME_DIR: {}", ret.unwrap_err());
        exit(1);
    }
    let ret = prepare_wayland(&ctx);
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
    let runtime_dir = env::var("XDG_RUNTIME_DIR").expect("Error resolving XDG_RUNTIME_DIR");
    EgoContext {
        cur_user,
        cur_uid: get_current_uid(),
        runtime_dir: runtime_dir.into(),
        target_user: username.to_string(),
        target_uid: user.uid(),
    }
}

fn add_file_acl(path: &Path, uid: u32, flags: u32) -> Result<(), SimpleError> {
    let mut acl = PosixACL::read_acl(path)?;
    acl.set(Qualifier::User(uid), flags);
    acl.write_acl(path)?;

    Ok(())
}

/// Add execute perm to runtime dir, e.g. `/run/user/1000`
fn prepare_runtime_dir(ctx: &EgoContext) -> Result<(), SimpleError> {
    let path = &ctx.runtime_dir;
    if !path.is_dir() {
        bail!("Path {} is not a directory", path.display());
    }
    add_file_acl(path, ctx.target_uid, ACL_EXECUTE)?;
    println!("Runtime data dir {} configured", path.display());
    Ok(())
}

fn get_wayland_socket(ctx: &EgoContext) -> Result<PathBuf, VarError> {
    let path = env::var("WAYLAND_DISPLAY")?;
    // May be full path or relative
    if path.starts_with('/') {
        Ok(path.into())
    } else {
        Ok(format!("{}/{}", ctx.runtime_dir.to_str().unwrap(), path).into())
    }
}

/// Add rwx permissions to Wayland socket (e.g. `/run/user/1000/wayland-0`)
fn prepare_wayland(ctx: &EgoContext) -> Result<(), SimpleError> {
    let path = get_wayland_socket(ctx);
    if path.is_err() {
        println!("Cannot detect Wayland socket: {}", path.err().unwrap());
        return Ok(());
    }
    let path = path.unwrap();
    add_file_acl(
        path.as_path(),
        ctx.target_uid,
        ACL_READ | ACL_WRITE | ACL_EXECUTE,
    )?;
    println!("Wayland socket {} configured", path.display());
    Ok(())
}
