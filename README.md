ego (a.k.a Alter Ego)
=====================

Ego is a tool to run Linux desktop applications under a different local user. Currently integrates
with Wayland, Xorg and PulseAudio.

Work in progress. :)

Setup
-----
The goal of ego is to come with sane defaults and be as easy as possible to set up. But it's not
entirely there yet.

Create the local user "ego" and add them to relevant user groups:

    sudo useradd ego --uid 155

(Low UID like 155 hides the user on the login screen)
