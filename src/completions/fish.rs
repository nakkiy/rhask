const DROP_TOP_LEVEL: &str = r#"complete -c rhask -n "__fish_rhask_needs_command" -f -a "complete-tasks" -d 'Internal helper for shell completions'
"#;

const DROP_SUBCOMMAND: &str = r#"complete -c rhask -n "__fish_rhask_using_subcommand complete-tasks" -s f -l file -d 'Path to the Rhai script file (defaults to searching for rhaskfile.rhai)' -r
complete -c rhask -n "__fish_rhask_using_subcommand complete-tasks" -s h -l help -d 'Print help'
"#;

const DYNAMIC_COMPLETIONS: &str = r#"

function __fish_rhask_task_candidates
    set -l prefix (commandline -ct)
    set -l tokens (commandline -opc)
    set -l file
    set -l idx 2
    set -l total (count $tokens)
    while test $idx -le $total
        set token $tokens[$idx]
        switch $token
            case '-f' '--file'
                set idx (math $idx + 1)
                if test $idx -le $total
                    set file $tokens[$idx]
                end
                set idx (math $idx + 1)
                continue
            case '--'
                break
        end
        set idx (math $idx + 1)
    end
    set -l cmd rhask
    if test -n "$file"
        set cmd $cmd --file $file
    end
    set cmd $cmd complete-tasks $prefix
    $cmd 2>/dev/null
end

function __fish_rhask_should_complete_tasks_direct
    __fish_rhask_needs_command
    or return 1
    set -l current_token (commandline -ct)
    test -n "$current_token"
    or return 1
    string match -q -- '-*' "$current_token"
    and return 1
    for reserved in list run completions
        test "$current_token" = $reserved
        and return 1
    end
    return 0
end

function __fish_rhask_should_complete_tasks_run
    __fish_rhask_using_subcommand run
    or return 1
    set -l tokens (commandline -opc)
    set -l total (count $tokens)
    set -l run_seen 0
    set -l idx 2
    set -l expect_value 0
    while test $idx -le $total
        set token $tokens[$idx]
        if test $run_seen -eq 0
            test "$token" = "run"
            and set run_seen 1
            set idx (math $idx + 1)
            continue
        end
        if test $expect_value -eq 1
            set expect_value 0
            set idx (math $idx + 1)
            continue
        end
        switch $token
            case '-f' '--file'
                set expect_value 1
            case '-*'
                ;
            case '*'
                return 1
        end
        set idx (math $idx + 1)
    end
    return 0
end

complete -c rhask -n "__fish_rhask_should_complete_tasks_direct" -a "(__fish_rhask_task_candidates)" -f
complete -c rhask -n "__fish_rhask_should_complete_tasks_run" -a "(__fish_rhask_task_candidates)" -f
"#;

pub fn patch(mut script: String) -> String {
    script = script.replace(DROP_TOP_LEVEL, "");
    script = script.replace(DROP_SUBCOMMAND, "");
    script.push_str(DYNAMIC_COMPLETIONS);
    script
}
