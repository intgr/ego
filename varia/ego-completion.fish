complete -c ego -s u -l user -d 'Specify a username (default: ego)' -r -f -a "(__fish_complete_users)"
complete -c ego -s h -l help -d 'Print help information'
complete -c ego -l sudo -d 'Use \'sudo\' to change user'
complete -c ego -l machinectl -d 'Use \'machinectl\' to change user (default, if available)'
complete -c ego -l machinectl-bare -d 'Use \'machinectl\' but skip xdg-desktop-portal setup'
complete -c ego -s v -l verbose -d 'Verbose output. Use multiple times for more output.'
