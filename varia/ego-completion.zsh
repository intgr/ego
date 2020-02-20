#compdef _ego ego
# To make this autocompletion discoverable by zsh, run:
#   sudo cp varia/ego-completion.zsh /usr/local/share/zsh/site-functions/_ego

function _ego {
    _arguments -C \
        {-h,--help}'[Show help information]' \
        {-u+,--user=}'[Specify a username]:user:_users' \
        {-v,--verbose}'[Verbose output]' \
        '--sudo[Execute using sudo]' \
        '--machinectl[Execute using machinectl]' \
        '--[]' \
        '(-)1:command: _command_names -e' \
        '*::arguments: _normal'
}

_ego
