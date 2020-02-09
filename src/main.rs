#[macro_use]
extern crate simple_error;

use std::env;
use std::env::VarError;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::exit;

use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_RWX};
use simple_error::SimpleError;
use users::{get_current_uid, get_current_username, get_user_by_name, uid_t};

type AnyErr = Box<dyn Error>;

struct EgoContext {
    #[allow(dead_code)] // FIXME
    cur_user: String,
    #[allow(dead_code)] // FIXME
    cur_uid: uid_t,
    runtime_dir: PathBuf,
    target_user: String,
    target_uid: uid_t,
}

fn main_inner() -> Result<(), AnyErr> {
    let username = "ego";
    let ctx = create_context(username)?;
    println!(
        "Setting up Alter Ego for user {} ({})",
        ctx.target_user, ctx.target_uid
    );

    let ret = prepare_runtime_dir(&ctx);
    if let Err(msg) = ret {
        bail!("Error preparing runtime dir: {}", msg);
    }
    let ret = prepare_wayland(&ctx);
    if let Err(msg) = ret {
        bail!("Error preparing Wayland: {}", msg)
    }
    Ok(())
}

fn main() {
    let ret = main_inner();
    if let Err(msg) = ret {
        eprintln!("Error: {}", msg);
        exit(1);
    }
}

/// Wrapper for nicer error messages
fn getenv(key: &str) -> Result<String, SimpleError> {
    match env::var(key) {
        Ok(val) => Ok(val),
        Err(VarError::NotPresent) => bail!("Env variable {} unset", key),
        // We could use Path type for non-Unicode paths, but it's not worth it. Fix your s*#t!
        Err(VarError::NotUnicode(_)) => bail!("Env variable {} invalid", key),
    }
}

fn create_context(username: &str) -> Result<EgoContext, AnyErr> {
    let cur_user = get_current_username()
        .expect("Unable to resolve current user")
        .into_string()
        .expect("Invalid current user username");
    let user = require_with!(get_user_by_name(&username), "Unknown user '{}'", username);
    let runtime_dir = getenv("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
        cur_user,
        cur_uid: get_current_uid(),
        runtime_dir: runtime_dir.into(),
        target_user: username.to_string(),
        target_uid: user.uid(),
    })
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
        bail!("'{}' is not a directory", path.display());
    }
    add_file_acl(path, ctx.target_uid, ACL_EXECUTE)?;
    println!("Runtime data dir '{}' configured", path.display());
    Ok(())
}

fn get_wayland_socket(ctx: &EgoContext) -> Result<PathBuf, AnyErr> {
    let path = getenv("WAYLAND_DISPLAY")?;
    // May be full path or relative
    if path.starts_with('/') {
        Ok(path.into())
    } else {
        Ok(format!("{}/{}", ctx.runtime_dir.to_str().unwrap(), path).into())
    }
}

/// Add rwx permissions to Wayland socket (e.g. `/run/user/1000/wayland-0`)
fn prepare_wayland(ctx: &EgoContext) -> Result<(), AnyErr> {
    let path = get_wayland_socket(ctx)?;
    add_file_acl(path.as_path(), ctx.target_uid, ACL_RWX)?;
    println!("Wayland socket '{}' configured", path.display());
    Ok(())
}
