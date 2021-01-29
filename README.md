ego (a.k.a Alter Ego)
=====================

[![Crates.io version](https://img.shields.io/crates/v/ego.svg)](https://crates.io/crates/ego)
[![Tests status](https://github.com/intgr/ego/workflows/Tests/badge.svg?branch=master)](https://github.com/intgr/ego/actions?query=workflow:Tests)

> Do all your games need access to your documents, browser history, SSH private keys?
>
> ... No? Just run `ego steam`!

**Ego** is a tool to run Linux desktop applications under a different local user. Currently
integrates with Wayland, Xorg and PulseAudio. You may think of it as `xhost` for Wayland and
PulseAudio. This is done using filesystem ACLs and `xhost` command.

Work in progress. :)

Disclaimer: **DO NOT RUN UNTRUSTED PROGRAMS VIA EGO.** However, using ego is more secure than
running applications directly under your primary user.

Installation
------------
The goal of ego is to come with sane defaults and be as easy as possible to set up.

1. Make sure you [have Rust installed](https://www.rust-lang.org/tools/install) and run:

       cargo install ego
       sudo cp ./.cargo/bin/ego /usr/local/bin/

2. Create local user named "ego": <sup>[1]</sup>

       sudo useradd ego --uid 155 --create-home

3. That's all, try it:

       ego xdg-open .

To avoid entering the password for sudo, add this to `/etc/sudoers` file (replace `<myname>` with
your own username):

    <myname> ALL=(ego) NOPASSWD:ALL

[1] No extra groups are necessary for typical usage. UID below 1000 hides this user on the login
    screen.

Changelog
---------
##### Unreleased
* Fix `--machinectl` on Ubuntu, Debian with dash shell (#42)
* Fix error reporting when command execution fails (#43)

##### 0.4.0 (2021-01-29)
* Improved integration with desktop environments:
  * Launch xdg-desktop-portal-gtk in machinectl session (#6, #31)
  * Old behavior is still available via `--machinectl-bare` switch.
* Shell completion files are now auto-generated with clap-generate 3.0.0-beta.2 (#36, #28)
* Code reorganization and CI improvements (#21, #23)
* Dependency updates (#20, #24, #27, #22, #26, #33, #35, #38, #37, #39)

##### 0.3.1 (2020-03-02)
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
