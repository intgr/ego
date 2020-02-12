ego (a.k.a Alter Ego)
=====================

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

        cargo build --release
        sudo cp ./target/release/ego /usr/local/bin/

2. Create local user named "ego": <sup>[1]</sup>

        sudo useradd ego --uid 155 --create-home

3. That's all, try it:

        ego xdg-open .

To avoid entering the password for sudo, add this to `/etc/sudoers` file (replace `<myname>` with
your own username):

    <myname> ALL=(ego) NOPASSWD:ALL

[1] No extra groups are necessary for typical usage. UID below 1000 hides this user on the login
    screen.

Metadata
--------
Ego is licensed under the MIT License (see the `LICENSE` file). Ego was created by Marti Raudsepp.
Ego's primary website is at https://github.com/intgr/ego
