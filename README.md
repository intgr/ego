ego (a.k.a Alter Ego)
=====================

[![Crates.io version](https://img.shields.io/crates/v/ego.svg)](https://crates.io/crates/ego)
[![Tests status](https://github.com/intgr/ego/workflows/Tests/badge.svg?branch=master)](https://github.com/intgr/ego/actions?query=workflow:Tests)

> Do all your games need access to your documents, browser history, SSH private keys?
>
> ... No? Just run `ego steam`!

**Ego** is a tool to run Linux desktop applications under a different local user. Currently
integrates with Wayland, Xorg, PulseAudio and xdg-desktop-portal. You may think of it as `xhost`
for Wayland and PulseAudio. This is done using filesystem ACLs and `xhost` command.

Disclaimer: **DO NOT RUN UNTRUSTED PROGRAMS VIA EGO.** However, using ego is more secure than
running applications directly under your primary user.

Distro packages
---------------
Distribution packages are available for:
* Arch Linux (user-contributed package) https://aur.archlinux.org/packages/ego/

After installing the package, add yourself to the `ego-users` group. After logout and login,
the `ego` command should just work.

([varia/README.md](varia/README.md) documents recommendations for distro packagers)

Manual setup
------------
Ego aims to come with sane defaults and be easy to set up.

**Requirements:**
* [Rust & cargo](https://www.rust-lang.org/tools/install)
* `libacl.so` library (Debian/Ubuntu: libacl1-dev; Fedora: libacl-devel; Arch: acl)
* `xhost` binary (Debian/Ubuntu: x11-xserver-utils; Fedora: xorg-xhost; Arch: xorg-xhost)

**Recommended:** (Not needed when using `--sudo` mode, but some desktop functionality may not work).
* `machinectl` command (Debian/Ubuntu/Fedora: systemd-container; Arch: systemd)
* `xdg-desktop-portal-gtk` (Debian/Ubuntu/Fedora/Arch: xdg-desktop-portal-gtk)

**Installation:**

1. Run:

       cargo install ego
       sudo cp ~/.cargo/bin/ego /usr/local/bin/

2. Create local user named "ego": <sup>[1]</sup>

       sudo useradd ego --uid 155 --create-home

3. That's all, try it:

       ego xdg-open .

[1] No extra groups are needed by the ego user.
UID below 1000 hides this user on the login screen.

### Avoid password prompt
If using "machinectl" mode (default if available), you need the rather new systemd version >=247
and polkit >=0.106 to do this securely.

Create file `/etc/polkit-1/rules.d/50-ego-machinectl.rules`, polkit will automatically load it
(replace `<myname>` with your own username):

```js
polkit.addRule(function(action, subject) {
    if (action.id == "org.freedesktop.machine1.host-shell" &&
        action.lookup("user") == "ego" &&
        subject.user == "<myname>") {
            return polkit.Result.YES;
    }
});
```

##### sudo mode
For sudo, add the following to `/etc/sudoers` (replace `<myname>` with your own username):

    <myname> ALL=(ego) NOPASSWD:ALL

Changelog
---------

##### 1.1.6 (2023-01-21)
* Updated to clap 4.0.x (#101) and many other dependency updates
* Fixes for new clippy lints (#95, #93, #111)
* Use `snapbox` instead of hand-coded snapshot testing (#102)
* MSRV was determined to be Rust 1.60.0 (#113)

##### 1.1.5 (2022-01-02)
* Document xhost requirement, improve xhost error reporting (#76)
* Upgrade to clap 3.0.0 stable (#71)

(Version 1.1.4 was yanked, it was accidentally released with a regression)

##### 1.1.3 (2021-11-12)
* Pin clap version (fixes #65) (#68)

##### 1.1.2 (2021-05-08)
* Enable sudo askpass helper if SUDO_ASKPASS is set (#58)
  * Example how to set up a GUI password prompt with sudo: https://askubuntu.com/a/314401
  * Note: For a GUI password prompt with the machinectl mode, you need to run a
    Polkit authentication agent instead

##### 1.1.1 (2021-03-23)
* Include drop-in files for polkit, sudoers.d, sysusers.d -- for distro packages (#53)
* Documentation tweaks (#51, #53)

##### 1.1.0 (2021-03-07)
* Default to `machinectl` if available, fall back to `sudo` otherwise (#47)
* Documentation & minor improvements (#46, #48)

##### 0.4.1 (2021-01-29)
* Fixed `--machinectl` on Ubuntu, Debian with dash shell (#42)
* Fixed error reporting when command execution fails (#43)
* Documented how to avoid password prompt with machinectl & other doc tweaks (#41)

##### 0.4.0 (2021-01-29)
* Improved integration with desktop environments:
  * Launch xdg-desktop-portal-gtk in machinectl session (#6, #31)
  * Old behavior is still available via `--machinectl-bare` switch.
* Shell completion files are now auto-generated with clap-generate 3.0.0-beta.2 (#36, #28)
  * bash, zsh and fish shells are supported out of the box.
* Code reorganization and CI improvements (#21, #23)
* Dependency updates (#20, #24, #27, #22, #26, #33, #35, #38, #37, #39)

##### 0.3.1 (2020-03-17)
* Improved error message for missing target user (#16)

##### 0.3.0 (2020-03-02)
* Initial machinectl support (using `--machinectl`) (#8)
* Updated: posix-acl (#9)

##### 0.2.0 (2020-02-17)
* Added zsh completion support (#5)
* Added `--verbose` flag (#4)
* Added `--user` argument and command-line parsing (#3)

##### 0.1.0 (2020-02-13)
Initial version

Appendix
--------
Ego is licensed under the MIT License (see the `LICENSE` file). Ego was created by Marti Raudsepp.
Ego's primary website is at https://github.com/intgr/ego

Thanks to Alexander Payne (myrrlyn) for relinquishing the unused "ego" crate name.
