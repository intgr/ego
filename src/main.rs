#[macro_use]
extern crate simple_error;

use crate::cli::{parse_args, Method};
use crate::errors::{print_error, AnyErr, ErrorWithHint};
use log::{debug, info};
use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX};
use simple_error::SimpleError;
use std::env::VarError;
use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};
use users::os::unix::UserExt;
use users::{get_user_by_name, get_user_by_uid, uid_t, User};

mod cli;
mod errors;
mod logging;
#[cfg(test)]
mod tests;

struct EgoContext {
    runtime_dir: PathBuf,
    target_user: String,
    target_uid: uid_t,
    target_user_shell: PathBuf,
}

fn main_inner() -> Result<(), AnyErr> {
    let args = parse_args(Box::new(env::args()));
    logging::init_with_level(args.log_level);

    let mut vars: Vec<String> = Vec::new();
    let ctx = create_context(args.user)?;

    info!(
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

    let ret = match args.method {
        Method::Sudo => run_sudo_command(&ctx, vars, args.command),
        Method::Machinectl => run_machinectl_command(&ctx, vars, args.command),
    };
    if let Err(msg) = ret {
        bail!("Error changing user: {}", msg);
    }

    Ok(())
}

fn main() {
    let ret = main_inner();
    if let Err(err) = ret {
        print_error(err);
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

/// Get details of *target* user; on error, formats a nice user-friendly message with instructions.
fn get_target_user(username: &str) -> Result<User, ErrorWithHint> {
    if let Some(user) = get_user_by_name(&username) {
        return Ok(user);
    }

    let mut hint = "Specify different user with --user= or create a new user".to_string();

    // Find a free UID for a helpful error message.
    // UIDs >=1000 are visible on login screen, so better avoid them.
    //
    // https://refspecs.linuxfoundation.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/uidrange.html
    // > The system User IDs from 100 to 499 should be reserved for dynamic allocation by system
    // > administrators and post install scripts using useradd.
    for uid in 150..=499 {
        if get_user_by_uid(uid).is_none() {
            hint = format!(
                "{} with the command:\n    sudo useradd '{}' --uid {} --create-home",
                hint, username, uid
            );
            break;
        }
    }

    Err(ErrorWithHint::new(
        format!("Unknown user '{}'", username),
        hint,
    ))
}

fn create_context(username: String) -> Result<EgoContext, AnyErr> {
    let user = get_target_user(&username)?;
    let runtime_dir = getenv_path("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
        runtime_dir,
        target_user: username,
        target_uid: user.uid(),
        target_user_shell: user.shell().into(),
    })
}

fn add_file_acl(path: &Path, uid: u32, flags: u32) -> Result<(), AnyErr> {
    let mut acl = PosixACL::read_acl(path)?;
    acl.set(Qualifier::User(uid), flags);
    acl.write_acl(path)?;
    Ok(())
}

/// Add execute perm to runtime dir, e.g. `/run/user/1000`
fn prepare_runtime_dir(ctx: &EgoContext) -> Result<(), AnyErr> {
    let path = &ctx.runtime_dir;
    if !path.is_dir() {
        bail!("'{}' is not a directory", path.display());
    }
    add_file_acl(path, ctx.target_uid, ACL_EXECUTE)?;
    debug!("Runtime data dir '{}' configured", path.display());
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
        debug!("Wayland: WAYLAND_DISPLAY not set, skipping");
        return Ok(vec![]);
    }

    let path = path.unwrap();
    add_file_acl(path.as_path(), ctx.target_uid, ACL_RWX)?;

    debug!("Wayland socket '{}' configured", path.display());
    Ok(vec![format!("WAYLAND_DISPLAY={}", path.to_str().unwrap())])
}

/// Detect `DISPLAY` and run `xhost` to grant permissions.
/// Return environment vars for `DISPLAY`
fn prepare_x11(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let display = getenv_optional("DISPLAY")?;
    if display.is_none() {
        debug!("X11: DISPLAY not set, skipping");
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

    debug!("X11 configured to allow {}", grant);
    Ok(vec![format!("DISPLAY={}", display.unwrap())])
}

/// Add execute permissions to PulseAudio directory (e.g. `/run/user/1000/pulse`)
/// Return environment vars for `PULSE_SERVER`.
///
/// The actual socket `/run/user/1000/pulse/native` already has full read-write permissions.
fn prepare_pulseaudio(ctx: &EgoContext) -> Result<Vec<String>, AnyErr> {
    let path = ctx.runtime_dir.join("pulse");
    if !path.is_dir() {
        debug!("PulseAudio dir '{}' not found, skipping", path.display());
        return Ok(vec![]);
    }
    add_file_acl(path.as_path(), ctx.target_uid, ACL_EXECUTE)?;

    let mut envs = prepare_pulseaudio_socket(path.as_path())?;
    envs.extend(prepare_pulseaudio_cookie(ctx)?);

    debug!("PulseAudio dir '{}' configured", path.display());
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
    debug!(
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

    info!("Running command: sudo {}", args.join(" "));
    Command::new("sudo").args(args).exec();

    Ok(())
}

fn machinectl_remote_command(remote_cmd: Vec<String>, envvars: Vec<String>) -> String {
    let mut cmd = String::new();

    // Split env variables by '=', to pass just their names
    let env_names = envvars
        .iter()
        .map(|v| v.split('=').next().expect("Unexpected data in envvars"));

    // Set environment variables in systemd
    cmd.push_str(&format!(
        "dbus-update-activation-environment --systemd {}; ",
        shell_words::join(env_names)
    ));
    // TODO: Should we support desktop-portals other than gtk?
    // XXX what happens if the desktop-portal is already running but with an outdated environment?
    cmd.push_str("systemctl --user start xdg-desktop-portal-gtk; ");
    cmd.push_str(&format!("exec -- {}", shell_words::join(remote_cmd)));
    return cmd;
}

fn run_machinectl_command(
    ctx: &EgoContext,
    envvars: Vec<String>,
    remote_cmd: Vec<String>,
) -> Result<(), AnyErr> {
    let mut args = vec!["shell".to_string()];
    args.push(format!("--uid={}", ctx.target_user));
    args.extend(envvars.iter().map(|v| format!("-E{}", v)));
    args.push("--".to_string());
    args.push(".host".to_string());

    // I wish this could be done without going through /bin/sh, but seems necessary.
    args.push("/bin/sh".to_string());
    args.push("-c".to_string());
    let remote_cmd = if remote_cmd.is_empty() {
        vec![require_with!(
            ctx.target_user_shell.to_str(),
            "User '{}' shell has unexpected characters",
            ctx.target_user
        )
        .to_string()]
    } else {
        remote_cmd
    };
    args.push(machinectl_remote_command(remote_cmd, envvars));

    info!("Running command: machinectl {}", shell_words::join(&args));
    Command::new("machinectl").args(args).exec();

    Ok(())
}
