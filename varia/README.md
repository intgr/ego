Shell completion
----------------
For shell completions to work, these files should be installed as:

* `ego-completion.zsh` → `/usr/share/zsh/site-functions/_ego`
* `ego-completion.bash` → `/usr/share/bash-completion/completions/ego`
* `ego-completion.fish` → `/usr/share/fish/vendor_completions.d/ego.fish`

These files are auto-generated with `clap_generate`. To update them, run
`cargo test --features=update-snapshots`

Packaging ego
-------------
The following files are helpful for distribution packagers, so ego can work seamlessly out of the box.

Distro packages should auto-create the `ego` user with low UID (<1000) and home `/home/ego`.
And a separate group `ego-users` for users that are allowed to invoke commands as `ego`.

The `ego.sysusers` drop-in file should create them on distros that support sysusers.d.
The sudoers and polkit rules files then permit switching users.

* `ego.sysusers.conf` → `/usr/lib/sysusers.d/ego.conf`
* `ego.sudoers` → `/etc/sudoers.d/50_ego`
* `ego.rules` → `/usr/share/polkit-1/rules.d/50-ego.rules`

Note: `ego.rules` requires systemd version >=247 and polkit >=0.106.
