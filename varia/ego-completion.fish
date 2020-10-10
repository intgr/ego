complete -c ego -n "__fish_use_subcommand" -s u -l user -d 'Specify a username (default: ego)' -r -f -a "(__fish_complete_users)"
complete -c ego -n "__fish_use_subcommand" -l sudo -d 'Use \'sudo\' to change user (default)'
complete -c ego -n "__fish_use_subcommand" -l machinectl -d 'Use \'machinectl\' to change user'
complete -c ego -n "__fish_use_subcommand" -l machinectl-bare -d 'Use \'machinectl\' but skip xdg-desktop-portal setup'
complete -c ego -n "__fish_use_subcommand" -s v -l verbose -d 'Verbose output. Use multiple times for more output.'
complete -c ego -n "__fish_use_subcommand" -s h -l help -d 'Prints help information'

