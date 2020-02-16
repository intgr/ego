#[macro_use]
extern crate simple_error;

use std::env::VarError;
use std::error::Error;
use std::ffi::OsString;
use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};

use clap::{App, AppSettings, Arg};
use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX};
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

struct Args {
    user: String,
    command: Vec<String>,
}

fn parse_args<T: Into<OsString> + Clone>(args: impl IntoIterator<Item = T>) -> Args {
    let matches = App::new("Alter Ego: run desktop applications under a different local user")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("user")
                .short("u")
                .long("user")
                .value_name("USER")
                .help("Specify a username (default: ego)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("command")
                .help("Command name and arguments to run (default: user shell)")
                .multiple(true),
        )
        .get_matches_from(args);

    Args {
        user: matches.value_of("user").unwrap_or("ego").to_string(),
        command: matches
            .values_of("command")
            .unwrap_or_default()
            .map(|v| v.to_string())
            .collect(),
    }
}

fn main_inner() -> Result<(), AnyErr> {
    let args = parse_args(Box::new(env::args()));
    let mut vars: Vec<String> = Vec::new();
    let ctx = create_context(args.user)?;
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

    if let Err(msg) = run_sudo_command(&ctx, vars, args.command) {
        bail!("Error running command: {}", msg);
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

fn create_context(username: String) -> Result<EgoContext, AnyErr> {
    let user = require_with!(get_user_by_name(&username), "Unknown user '{}'", username);
    let runtime_dir = getenv_path("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
        runtime_dir,
        target_user: username,
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

    let mut envs = prepare_pulseaudio_socket(path.as_path())?;
    envs.extend(prepare_pulseaudio_cookie(ctx)?);

    println!("PulseAudio dir '{}' configured", path.display());
    Ok(envs)
}

/// Ensure permissions of PulseAudio socket `/run/user/1000/pulse/native`
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

/// Try various ways to discover the current user's PulseAudio authentication cookie.
fn find_pulseaudio_cookie() -> Result<PathBuf, AnyErr> {
    // Try PULSE_COOKIE
    if let Some(path) = getenv_optional("PULSE_COOKIE")? {
        return Ok(PathBuf::from(path));
    }
    // Try ~/.config/pulse/cookie
    let home = getenv_path("HOME")?;
    let path = home.join(".config/pulse/cookie");
    if path.is_file() {
        return Ok(path);
    }

    // Try ~/.pulse-cookie, for older PulseAudio versions
    let path = home.join(".pulse-cookie");
    if path.is_file() {
        return Ok(path);
    }

    bail!(
        "Cannot locate PulseAudio cookie \
        (tried $PULSE_COOKIE, ~/.config/pulse/cookie, ~/.pulse-cookie)"
    )
}

/// Publish current user's pulse-cookie for target user
fn prepare_pulseaudio_cookie(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let cookie_path = find_pulseaudio_cookie()?;
    let target_path = ensure_ego_rundir(ctx)?.join("pulse-cookie");
    println!(
        "Publishing PulseAudio cookie {} to {}",
        cookie_path.display(),
        target_path.display()
    );
    fs::copy(cookie_path.as_path(), target_path.as_path())?;
    add_file_acl(target_path.as_path(), ctx.target_uid, ACL_READ)?;

    Ok(vec![format!(
        "PULSE_COOKIE={}",
        target_path.to_str().unwrap()
    )])
}

/// Create runtime dir for Ego itself (e.g. `/run/user/1000/ego`) and make it readable for target
/// user. This directory us used to share state (e.g. PulseAudio auth cookie).
fn ensure_ego_rundir(ctx: &EgoContext) -> Result<PathBuf, AnyErr> {
    // XXX We assume that prepare_runtime_dir() has already been called.
    let path = ctx.runtime_dir.join("ego");
    if !path.is_dir() {
        DirBuilder::new().mode(0o700).create(path.as_path())?;
    }
    // Set ACL either way, because target user may be different in every run.
    add_file_acl(path.as_path(), ctx.target_uid, ACL_EXECUTE)?;
    Ok(path)
}

fn run_sudo_command(
    ctx: &EgoContext,
    envvars: Vec<String>,
    remote_cmd: Vec<String>,
) -> Result<(), AnyErr> {
    if !remote_cmd.is_empty() && remote_cmd[0].starts_with('-') {
        bail!(
            "Command may not start with '-' (command is: '{}')",
            remote_cmd[0]
        );
    }

    let mut args = vec!["-SHiu".to_string(), ctx.target_user.clone()];
    args.extend(envvars);
    args.extend(remote_cmd);

    println!("Running command: sudo {}", args.join(" "));
    Command::new("sudo").args(args).exec();

    Ok(())
}
