#[macro_use]
extern crate simple_error;

use std::env;
use std::env::VarError;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::exit;

use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_RWX};
use simple_error::SimpleError;
use users::{get_current_uid, get_current_username, get_user_by_name, uid_t};

type AnyErr = Box<dyn Error>;

#[cfg(test)]
mod tests;

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
    let ret = prepare_pulseaudio(&ctx);
    if let Err(msg) = ret {
        bail!("Error preparing PulseAudio: {}", msg)
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
fn getenv_path(key: &str) -> Result<PathBuf, SimpleError> {
    match env::var(key) {
        Ok(val) => Ok(val.into()),
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
    let runtime_dir = getenv_path("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
        cur_user,
        cur_uid: get_current_uid(),
        runtime_dir,
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

/// WAYLAND_DISPLAY may be absolute path or relative to XDG_RUNTIME_DIR
/// See https://manpages.debian.org/experimental/libwayland-doc/wl_display_connect.3.en.html
fn get_wayland_socket(ctx: &EgoContext) -> Result<PathBuf, AnyErr> {
    let display = getenv_path("WAYLAND_DISPLAY")?;
    Ok(ctx.runtime_dir.join(display))
}

/// Add rwx permissions to Wayland socket (e.g. `/run/user/1000/wayland-0`)
fn prepare_wayland(ctx: &EgoContext) -> Result<(), AnyErr> {
    let path = get_wayland_socket(ctx)?;
    add_file_acl(path.as_path(), ctx.target_uid, ACL_RWX)?;
    println!("Wayland socket '{}' configured", path.display());
    Ok(())
}

/// Add execute permissions to PulseAudio directory (e.g. `/run/user/1000/pulse`)
///
/// The actual socket `/run/user/1000/pulse/native` already has full read-write permissions.
fn prepare_pulseaudio(ctx: &EgoContext) -> Result<(), AnyErr> {
    let path = ctx.runtime_dir.join("pulse");
    if !path.is_dir() {
        println!("PulseAudio dir '{}' not found, skipping", path.display());
        return Ok(());
    }
    add_file_acl(path.as_path(), ctx.target_uid, ACL_EXECUTE)?;

    prepare_pulseaudio_socket(path.as_path())?;

    println!("PulseAudio dir '{}' configured", path.display());
    Ok(())
}

/// Check permissions of PulseAudio socket `/run/user/1000/pulse/native`
fn prepare_pulseaudio_socket(dir: &Path) -> Result<(), AnyErr> {
    let path = dir.join("native");
    let meta = path.metadata();
    if let Err(msg) = meta {
        bail!("'{}': {}", path.display(), msg);
    }
    let mode = meta.unwrap().permissions().mode();
    const WORLD_READ_PERMS: u32 = 0o006;
    if mode & WORLD_READ_PERMS != WORLD_READ_PERMS {
        bail!(
            "Unexpected permissions on '{}': {:o}",
            path.display(),
            mode & 0o777
        );
    }
    Ok(())
}
