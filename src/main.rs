#[macro_use]
extern crate simple_error;

use std::env;
use std::env::VarError;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_RWX};
use simple_error::SimpleError;
use users::{get_user_by_name, uid_t};

type AnyErr = Box<dyn Error>;

#[cfg(test)]
mod tests;

struct EgoContext {
    runtime_dir: PathBuf,
    target_user: String,
    target_uid: uid_t,
}

fn main_inner() -> Result<(), AnyErr> {
    let mut vars: Vec<String> = Vec::new();
    let username = "ego"; // TODO: take username as argument
    let ctx = create_context(username)?;
    println!(
        "Setting up Alter Ego for user {} ({})",
        ctx.target_user, ctx.target_uid
    );

    let ret = prepare_runtime_dir(&ctx);
    if let Err(msg) = ret {
        bail!("Error preparing runtime dir: {}", msg);
    }
    match prepare_wayland(&ctx) {
        Err(msg) => bail!("Error preparing Wayland: {}", msg),
        Ok(ret) => vars.extend(ret),
    }
    match prepare_x11(&ctx) {
        Err(msg) => bail!("Error preparing X11: {}", msg),
        Ok(ret) => vars.extend(ret),
    }
    match prepare_pulseaudio(&ctx) {
        Err(msg) => bail!("Error preparing PulseAudio: {}", msg),
        Ok(ret) => vars.extend(ret),
    }
    // TODO: Set up xdg-desktop-portal-gtk

    run_sudo_command(&ctx, vars, env::args().skip(1).collect());

    Ok(())
}

fn main() {
    let ret = main_inner();
    if let Err(msg) = ret {
        eprintln!("Error: {}", msg);
        exit(1);
    }
}

/// Optionally get an environment variable.
/// Returns `Ok(None)` for missing env variable.
fn getenv_optional(key: &str) -> Result<Option<String>, SimpleError> {
    match env::var(key) {
        Ok(val) => Ok(Some(val)),
        Err(VarError::NotPresent) => Ok(None),
        // We could use Path type for non-Unicode paths, but it's not worth it. Fix your s*#t!
        Err(VarError::NotUnicode(_)) => bail!("Env variable {} invalid", key),
    }
}

/// Require an environment variable.
fn getenv_path(key: &str) -> Result<PathBuf, SimpleError> {
    match getenv_optional(key)? {
        Some(val) => Ok(PathBuf::from(val)),
        None => bail!("Env variable {} unset", key),
    }
}

fn create_context(username: &str) -> Result<EgoContext, AnyErr> {
    let user = require_with!(get_user_by_name(&username), "Unknown user '{}'", username);
    let runtime_dir = getenv_path("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
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
fn get_wayland_socket(ctx: &EgoContext) -> Result<Option<PathBuf>, AnyErr> {
    match getenv_optional("WAYLAND_DISPLAY")? {
        None => Ok(None),
        Some(display) => Ok(Some(ctx.runtime_dir.join(display))),
    }
}

/// Add rwx permissions to Wayland socket (e.g. `/run/user/1000/wayland-0`)
/// Return environment vars for `WAYLAND_DISPLAY`.
fn prepare_wayland(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let path = get_wayland_socket(ctx)?;
    if path.is_none() {
        println!("Wayland: WAYLAND_DISPLAY not set, skipping");
        return Ok(vec![]);
    }

    let path = path.unwrap();
    add_file_acl(path.as_path(), ctx.target_uid, ACL_RWX)?;

    println!("Wayland socket '{}' configured", path.display());
    Ok(vec![format!("WAYLAND_DISPLAY={}", path.to_str().unwrap())])
}

/// Detect `DISPLAY` and run `xhost` to grant permissions.
/// Return environment vars for `DISPLAY`
fn prepare_x11(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let display = getenv_optional("DISPLAY")?;
    if display.is_none() {
        println!("X11: DISPLAY not set, skipping");
        return Ok(vec![]);
    }

    let grant = format!("+si:localuser:{}", ctx.target_user);
    let ret = Command::new("xhost").arg(&grant).output()?;
    if !ret.status.success() {
        bail!(
            "xhost returned {}:\n{}",
            ret.status.code().unwrap_or(999),
            String::from_utf8_lossy(&ret.stderr)
        );
    }
    // TODO should also test /tmp/.X11-unix/X0 permissions?

    println!("X11 configured to allow {}", grant);
    Ok(vec![format!("DISPLAY={}", display.unwrap())])
}

/// Add execute permissions to PulseAudio directory (e.g. `/run/user/1000/pulse`)
/// Return environment vars for `PULSE_SERVER`.
///
/// The actual socket `/run/user/1000/pulse/native` already has full read-write permissions.
fn prepare_pulseaudio(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let path = ctx.runtime_dir.join("pulse");
    if !path.is_dir() {
        println!("PulseAudio dir '{}' not found, skipping", path.display());
        return Ok(vec![]);
    }
    add_file_acl(path.as_path(), ctx.target_uid, ACL_EXECUTE)?;

    let envs = prepare_pulseaudio_socket(path.as_path())?;
    // TODO: Automatically set up PulseAudio cookie

    println!("PulseAudio dir '{}' configured", path.display());
    Ok(envs)
}

/// Check permissions of PulseAudio socket `/run/user/1000/pulse/native`
fn prepare_pulseaudio_socket(dir: &Path) -> Result<Vec<String>, AnyErr> {
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
    Ok(vec![format!(
        "PULSE_SERVER=unix:{}",
        path.to_str().unwrap()
    )])
}

fn run_sudo_command(ctx: &EgoContext, envvars: Vec<String>, remote_cmd: Vec<String>) {
    let mut args = vec!["-SHiu".to_string(), ctx.target_user.clone()];
    args.extend(envvars);
    args.extend(remote_cmd);

    println!("Running command: sudo {}", args.join(" "));
    Command::new("sudo").args(args).exec();
}
