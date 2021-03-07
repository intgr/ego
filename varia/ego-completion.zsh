#compdef ego

autoload -U is-at-least

_ego() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" \
'-u+[Specify a username (default: ego)]: :_users' \
'--user=[Specify a username (default: ego)]: :_users' \
'--sudo[Use '\''sudo'\'' to change user]' \
'--machinectl[Use '\''machinectl'\'' to change user (default, if available)]' \
'--machinectl-bare[Use '\''machinectl'\'' but skip xdg-desktop-portal setup]' \
'*-v[Verbose output. Use multiple times for more output.]' \
'*--verbose[Verbose output. Use multiple times for more output.]' \
'-h[Prints help information]' \
'--help[Prints help information]' \
'*::command -- Command name and arguments to run (default\: user shell):_cmdambivalent' \
&& ret=0
    
}

(( $+functions[_ego_commands] )) ||
_ego_commands() {
    local commands; commands=(
        
    )
    _describe -t commands 'ego commands' commands "$@"
}

_ego "$@"
