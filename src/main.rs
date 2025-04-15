#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::multiple_crate_versions)]

#[macro_use]
extern crate simple_error;

use crate::cli::{parse_args, Method};
use crate::errors::{print_error, AnyErr, ErrorWithHint};
use crate::util::{exec_command, have_command, run_command, sd_booted};
use crate::x11::x11_add_acl;
use log::{debug, info, log, warn, Level};
use nix::libc::uid_t;
use nix::unistd::{Uid, User};
use posix_acl::{PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX};
use simple_error::SimpleError;
use std::env::VarError;
use std::fs::DirBuilder;
use std::io::ErrorKind::PermissionDenied;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{env, fs};

mod cli;
mod errors;
mod logging;
#[cfg(test)]
mod tests;
mod util;
mod x11;

#[derive(Clone)]
struct EgoContext {
    runtime_dir: PathBuf,
    target_user: String,
    target_uid: uid_t,
    target_user_shell: PathBuf,
    target_user_homedir: PathBuf,
}

fn main_inner() -> Result<(), AnyErr> {
    let args = parse_args(env::args());
    logging::init_with_level(args.log_level);

    let mut vars: Vec<String> = Vec::new();
    let ctx = create_context(&args.user)?;

    info!(
        "Setting up Alter Ego for target user {} ({})",
        ctx.target_user, ctx.target_uid
    );

    check_user_homedir(&ctx);

    let ret = prepare_runtime_dir(&ctx);
    if let Err(msg) = ret {
        bail!("Error preparing runtime dir: {msg}");
    }
    match prepare_wayland(&ctx) {
        Err(msg) => bail!("Error preparing Wayland: {msg}"),
        Ok(ret) => vars.extend(ret),
    }
    match prepare_x11(&ctx, args.old_xhost) {
        Err(msg) => bail!("Error preparing X11: {msg}"),
        Ok(ret) => vars.extend(ret),
    }
    match prepare_pulseaudio(&ctx) {
        Err(msg) => bail!("Error preparing PulseAudio: {msg}"),
        Ok(ret) => vars.extend(ret),
    }

    let method = args.method.unwrap_or_else(detect_method);
    let ret = match method {
        Method::Sudo => run_sudo_command(&ctx, vars, args.command),
        Method::Machinectl => run_machinectl_command(&ctx, &vars, args.command, false),
        Method::MachinectlBare => run_machinectl_command(&ctx, &vars, args.command, true),
    };
    if let Err(msg) = ret {
        bail!("{msg}");
    }

    Ok(())
}

fn main() {
    let ret = main_inner();
    if let Err(err) = ret {
        print_error(&err);
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
        Err(VarError::NotUnicode(_)) => bail!("Env variable {key} invalid"),
    }
}

/// Require an environment variable.
fn getenv_path(key: &str) -> Result<PathBuf, SimpleError> {
    match getenv_optional(key)? {
        Some(val) => Ok(PathBuf::from(val)),
        None => bail!("Env variable {key} unset"),
    }
}

/// Get details of *target* user; on error, formats a nice user-friendly message with instructions.
fn get_target_user(username: &str) -> Result<User, AnyErr> {
    if let Some(user) = User::from_name(username)? {
        return Ok(user);
    }

    debug!("Username '{username}' not found");

    let mut hint = "Specify different user with --user= or create a new user".to_string();

    // Find a free UID for a helpful error message.
    // UIDs >=1000 are visible on login screen, so better avoid them.
    //
    // https://refspecs.linuxfoundation.org/LSB_5.0.0/LSB-Core-generic/LSB-Core-generic/uidrange.html
    // > The system User IDs from 100 to 499 should be reserved for dynamic allocation by system
    // > administrators and post install scripts using useradd.
    for uid in 150..=499 {
        if User::from_uid(Uid::from_raw(uid))?.is_none() {
            hint = format!(
                "{hint} with the command:\n    sudo useradd '{username}' --uid {uid} --create-home"
            );
            break;
        }
        debug!("User UID {uid} already exists");
    }

    Err(ErrorWithHint::new(format!("Unknown user '{username}'"), hint).into())
}

fn create_context(username: &str) -> Result<EgoContext, AnyErr> {
    let user = get_target_user(username)?;
    debug!(
        "Found user '{}' UID {} shell '{}'",
        user.name,
        user.uid,
        user.shell.display()
    );
    let runtime_dir = getenv_path("XDG_RUNTIME_DIR")?;
    Ok(EgoContext {
        runtime_dir,
        target_user: user.name,
        target_uid: user.uid.as_raw(),
        target_user_shell: user.shell,
        target_user_homedir: user.dir,
    })
}

fn add_file_acl(path: &Path, uid: u32, flags: u32) -> Result<(), AnyErr> {
    let mut acl = PosixACL::read_acl(path)?;
    acl.set(Qualifier::User(uid), flags);
    acl.write_acl(path)?;
    Ok(())
}

/// Report warning if user home directory does not exist or has wrong ownership
fn check_user_homedir(ctx: &EgoContext) {
    let home = &ctx.target_user_homedir;
    match fs::metadata(home) {
        Ok(meta) => {
            if meta.uid() != ctx.target_uid {
                warn!(
                    "User {} home directory {} has incorrect ownership (expected UID {}, found {})",
                    ctx.target_user,
                    home.display(),
                    ctx.target_uid,
                    meta.uid()
                );
            }
        }
        Err(err) => {
            // Report PermissionDenied as `info` level, user home directory is probably in a parent
            // directory we have no access to, avoid nagging.
            let level = match err.kind() {
                PermissionDenied => Level::Info,
                _ => Level::Warn,
            };

            log!(
                level,
                "User {} home directory {} is not accessible: {err}",
                ctx.target_user,
                home.display(),
            );
        }
    }
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

/// `WAYLAND_DISPLAY` may be absolute path or relative to `XDG_RUNTIME_DIR`
/// See <https://manpages.debian.org/experimental/libwayland-doc/wl_display_connect.3.en.html>
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

/// Detect `DISPLAY` and grant permissions via X11 protocol `ChangeHosts` command
/// (or run `xhost` command if `--old-xhost` was used).
/// Return environment vars for `DISPLAY`
fn prepare_x11(ctx: &EgoContext, old_xhost: bool) -> Result<Vec<String>, AnyErr> {
    let display = getenv_optional("DISPLAY")?;
    if display.is_none() {
        debug!("X11: DISPLAY not set, skipping");
        return Ok(vec![]);
    }

    if old_xhost {
        warn!("--old-xhost is deprecated. If there are issues with the new method, please report a bug.");
        let grant = format!("+si:localuser:{}", ctx.target_user);
        run_command("xhost", &[grant])?;
    } else {
        x11_add_acl("localuser", &ctx.target_user)?;
    }
    // TODO should also test /tmp/.X11-unix/X0 permissions?

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
        bail!("'{}': {msg}", path.display());
    }
    let mode = meta.unwrap().permissions().mode();

    #[allow(clippy::items_after_statements)]
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

/// Detect which method should be used
fn detect_method() -> Method {
    if !sd_booted() {
        return Method::Sudo;
    }
    if !have_command("machinectl") {
        // If booted using systemd, issue a warning
        warn!("machinectl (systemd-container) is not installed");
        warn!("Falling back to 'sudo', some desktop integration features may not work");
        return Method::Sudo;
    }
    Method::Machinectl
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

    let mut args = vec!["-Hiu".to_string(), ctx.target_user.clone()];
    // If SUDO_ASKPASS envvar is set, add -A argument to use the askpass agent
    if let Ok(Some(_)) = getenv_optional("SUDO_ASKPASS") {
        debug!("SUDO_ASKPASS detected");
        args.push("-A".into());
    }
    args.extend(envvars);
    args.extend(remote_cmd);

    info!("Running command: sudo {}", args.join(" "));
    exec_command("sudo", &args)?;
    Ok(())
}

#[allow(clippy::format_push_string)]
fn machinectl_remote_command(remote_cmd: Vec<String>, envvars: &[String], bare: bool) -> String {
    let mut cmd = String::new();

    if !bare {
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
    }
    cmd.push_str(&format!("exec {}", shell_words::join(remote_cmd)));
    cmd
}

fn run_machinectl_command(
    ctx: &EgoContext,
    envvars: &[String],
    remote_cmd: Vec<String>,
    bare: bool,
) -> Result<(), AnyErr> {
    let mut args = vec!["shell".to_string()];
    args.push(format!("--uid={}", ctx.target_user));
    args.extend(envvars.iter().map(|v| format!("-E{v}")));
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
    args.push(machinectl_remote_command(remote_cmd, envvars, bare));

    info!("Running command: machinectl {}", shell_words::join(&args));
    exec_command("machinectl", &args)?;
    Ok(())
}
