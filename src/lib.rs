#![doc = include_str!("../README.md")]

pub mod cli;
pub mod engine;
pub mod logger;
pub mod printer;
pub mod task;

use clap::{CommandFactory, Parser};
use cli::Cli;
use logger::*;
use rhai::{EvalAltResult, Position};
use std::io;

pub fn run() -> Result<(), Box<EvalAltResult>> {
    let cli = Cli::parse();
    run_with_cli(cli)
}

pub fn run_with_cli(cli: Cli) -> Result<(), Box<EvalAltResult>> {
    logger::init();
    info!("start");
    debug!("cli args: {:?}", cli);

    let script_path = cli
        .file
        .clone()
        .unwrap_or_else(|| "rhaskfile.rhai".to_string());

    match cli.cmd {
        Some(cli::Commands::Completions(opts)) => {
            print_shell_completions(opts.shell);
            info!("{} end", env!("CARGO_PKG_NAME"));
            Ok(())
        }
        other => {
            let mut script_engine = engine::ScriptEngine::new();
            script_engine.run_script(&script_path)?;
            dispatcher(other, script_engine)?;
            info!("{} end", env!("CARGO_PKG_NAME"));
            Ok(())
        }
    }
}

fn dispatcher(
    cmd: Option<cli::Commands>,
    engine: engine::ScriptEngine,
) -> Result<(), Box<EvalAltResult>> {
    debug!("dispatching command: {:?}", cmd);
    match cmd {
        Some(cli::Commands::List(opts)) => {
            info!("Listing tasks: group={:?}, flat={}", opts.group, opts.flat);
            engine.list_tasks(opts.group.as_deref(), opts.flat);
            Ok(())
        }
        Some(cli::Commands::Run(opts)) => run_with_logging(engine, &opts.task, &opts.args),
        Some(cli::Commands::CompleteTasks(opts)) => {
            print_task_candidates(&engine, opts.prefix.as_deref().unwrap_or_default());
            Ok(())
        }
        Some(cli::Commands::Completions(_)) => unreachable!("handled earlier in run_with_cli"),
        Some(cli::Commands::Direct(raw)) => {
            let (task, args) = raw.split_first().ok_or_else(|| {
                warn!("Direct command invoked without a task name");
                missing_task_name_error()
            })?;
            run_with_logging(engine, task, args)
        }
        None => {
            if let Some(task) = engine.default_task() {
                run_with_logging(engine, &task, &[])
            } else {
                info!("Listing tasks: group=None, flat=false");
                engine.list_tasks(None, false);
                Ok(())
            }
        }
    }
}

fn run_with_logging(
    engine: engine::ScriptEngine,
    task: &str,
    args: &[String],
) -> Result<(), Box<EvalAltResult>> {
    info!("Executing task '{}'", task);
    if !args.is_empty() {
        debug!("Task '{}' arguments: {:?}", task, args);
    }
    engine.run_task(task, args).map_err(|err| {
        error!("failed to execute command: {}", err);
        err
    })
}

fn missing_task_name_error() -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        "Task name is required when omitting the 'run' subcommand."
            .to_string()
            .into(),
        Position::NONE,
    ))
}

fn print_shell_completions(shell: clap_complete::Shell) {
    use clap_complete::generate;
    let mut cmd = Cli::command().allow_external_subcommands(false);
    let bin_name = cmd.get_name().to_string();
    match shell {
        clap_complete::Shell::Bash => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", patch_bash_completion(script));
        }
        clap_complete::Shell::Zsh => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", patch_zsh_completion(script));
        }
        clap_complete::Shell::Fish => {
            let mut buffer = Vec::new();
            generate(shell, &mut cmd, &bin_name, &mut buffer);
            let script = String::from_utf8(buffer).expect("completions should be valid UTF-8");
            print!("{}", patch_fish_completion(script));
        }
        other => {
            generate(other, &mut cmd, &bin_name, &mut io::stdout());
        }
    }
}

fn patch_bash_completion(mut script: String) -> String {
    if script.contains("_rhask()") {
        script = script.replacen("_rhask()", "__rhask_base()", 1);
        script.push_str(BASH_DYNAMIC_COMPLETIONS);
        script.push_str(BASH_COMPLETION_REREGISTER);
    }
    script
}

fn patch_zsh_completion(mut script: String) -> String {
    if script.contains("_rhask()") {
        script = script.replacen("_rhask()", "__rhask_base()", 1);
        script = script.replace(ZSH_COMPLETION_REGISTRATION, "");
        script.push_str(ZSH_DYNAMIC_COMPLETIONS);
    }
    script
}

fn patch_fish_completion(mut script: String) -> String {
    script = script.replace(FISH_COMPLETE_TASKS_TOP_LEVEL, "");
    script = script.replace(FISH_COMPLETE_TASKS_SUBCOMMAND, "");
    script.push_str(FISH_DYNAMIC_COMPLETIONS);
    script
}

const BASH_DYNAMIC_COMPLETIONS: &str = r#"

__rhask_dynamic_tasks() {
    local current_word="${1:-}"
    local prefix="$current_word"
    local file=""
    local args=()
    local skip_next=0
    local idx=1
    while [[ $idx -lt ${#COMP_WORDS[@]} ]]; do
        local token="${COMP_WORDS[$idx]}"
        if [[ $skip_next -eq 1 ]]; then
            skip_next=0
            ((idx++))
            continue
        fi
        case "$token" in
            complete-tasks)
                :
                ;;
            --file|-f)
                ((idx++))
                file="${COMP_WORDS[$idx]}"
                ;;
            --)
                break
                ;;
            *)
                if [[ -z "$current_word" && "$token" == "${COMP_WORDS[COMP_CWORD]}" ]]; then
                    prefix="${COMP_WORDS[COMP_CWORD]}"
                elif [[ "$token" == "$current_word" ]]; then
                    prefix="$current_word"
                else
                    args+=("$token")
                fi
                ;;
        esac
        ((idx++))
    done
    local cmd=(rhask)
    if [[ -n "$file" ]]; then
        cmd+=(--file "$file")
    fi
    cmd+=(complete-tasks "$prefix")
    "${cmd[@]}" 2>/dev/null
}

_rhask() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local first_cmd_idx=1
    local idx=1
    local expect_value=0
    local found_first=0
    while [[ $idx -lt ${#COMP_WORDS[@]} ]]; do
        local token="${COMP_WORDS[$idx]}"
        if [[ $expect_value -eq 1 ]]; then
            expect_value=0
            ((idx++))
            continue
        fi
        case "$token" in
            --)
                first_cmd_idx=$((idx + 1))
                found_first=1
                break
                ;;
            -f|--file)
                expect_value=1
                ;;
            -*)
                ;;
            *)
                first_cmd_idx=$idx
                found_first=1
                break
                ;;
        esac
        ((idx++))
    done
    if [[ $found_first -eq 0 ]]; then
        first_cmd_idx=$idx
    fi

    if [[ ${COMP_CWORD} -eq $first_cmd_idx ]]; then
        case "$cur" in
            -*|list|run|completions|"")
                ;;
            *)
                local dynamic_candidates="$(__rhask_dynamic_tasks "$cur")"
                if [[ -n "$dynamic_candidates" ]]; then
                    COMPREPLY=( $(compgen -W "$dynamic_candidates" -- "$cur") )
                    return 0
                fi
                ;;
        esac
    elif [[ ${COMP_WORDS[1]} == run && ${COMP_CWORD} -eq 2 ]]; then
        case "$cur" in
            -*|"")
                ;;
            *)
                local candidates="$(__rhask_dynamic_tasks "$cur")"
                if [[ -n "$candidates" ]]; then
                    COMPREPLY=( $(compgen -W "$candidates" -- "$cur") )
                    return 0
                fi
                ;;
        esac
    fi
    __rhask_base "$@"
}
"#;

const BASH_COMPLETION_REREGISTER: &str = r#"
if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _rhask -o nosort -o bashdefault -o default rhask
else
    complete -F _rhask -o bashdefault -o default rhask
fi
"#;

fn print_task_candidates(engine: &engine::ScriptEngine, prefix: &str) {
    use std::io::Write;

    let mut entries: Vec<String> = {
        let registry = engine.registry.lock().unwrap();
        let mut names: Vec<String> = registry
            .tasks_iter()
            .map(|(name, _)| name.clone())
            .collect();
        names.extend(registry.groups_iter().map(|(name, _)| name.clone()));
        names
    };

    entries.sort();
    entries.dedup();

    let mut stdout = io::BufWriter::new(std::io::stdout());
    for name in entries.into_iter().filter(|name| name.starts_with(prefix)) {
        let _ = writeln!(stdout, "{name}");
    }
    let _ = stdout.flush();
}

const ZSH_COMPLETION_REGISTRATION: &str = r#"if [ "$funcstack[1]" = "_rhask" ]; then
    _rhask "$@"
else
    compdef _rhask rhask
fi
"#;

const ZSH_DYNAMIC_COMPLETIONS: &str = r#"

__rhask_dynamic_tasks() {
    local prefix="$1"
    local file=""
    local total=${#words[@]}
    local idx=2
    local expect_value=0
    while (( idx <= total )); do
        local token="${words[idx]}"
        if (( expect_value )); then
            expect_value=0
            file="$token"
            ((idx++))
            continue
        fi
        case "$token" in
            -f|--file)
                expect_value=1
                ;;
        esac
        ((idx++))
    done
    local -a cmd
    cmd=("rhask")
    if [[ -n "$file" ]]; then
        cmd+=("--file" "$file")
    fi
    cmd+=("complete-tasks" "$prefix")
    "${cmd[@]}" 2>/dev/null
}

_rhask() {
    local cur="${words[CURRENT]}"
    local idx=2
    local total=${#words[@]}
    local expect_value=0
    local first_idx=0
    local found_first=0
    while (( idx <= total )); do
        local token="${words[idx]}"
        if (( expect_value )); then
            expect_value=0
            ((idx++))
            continue
        fi
        case "$token" in
            --)
                first_idx=$((idx + 1))
                found_first=1
                break
                ;;
            -f|--file)
                expect_value=1
                ;;
            -*)
                ;;
            *)
                first_idx=$idx
                found_first=1
                break
                ;;
        esac
        ((idx++))
    done
    if (( found_first == 0 )); then
        first_idx=$((CURRENT))
    fi

    local handled=0
    if (( CURRENT == first_idx )); then
        case "$cur" in
            ""|-*|list|run|completions)
                ;;
            *)
                local -a dynamic
                dynamic=( ${(f)$(__rhask_dynamic_tasks "$cur")} )
                if (( ${#dynamic[@]} )); then
                    local expl
                    _wanted rhask-tasks expl 'task name' compadd -a dynamic
                    handled=1
                fi
                ;;
        esac
    elif (( first_idx > 0 )) && [[ "${words[first_idx]}" = run ]] && (( CURRENT == first_idx + 1 )); then
        local -a dynamic
        dynamic=( ${(f)$(__rhask_dynamic_tasks "$cur")} )
        if (( ${#dynamic[@]} )); then
            local expl
            _wanted rhask-run-tasks expl 'task name' compadd -a dynamic
            handled=1
        fi
    fi

    if (( handled )); then
        return 0
    fi

    __rhask_base "$@"
}

if [ "$funcstack[1]" = "_rhask" ]; then
    _rhask "$@"
else
    compdef _rhask rhask
fi
"#;

const FISH_COMPLETE_TASKS_TOP_LEVEL: &str = r#"complete -c rhask -n "__fish_rhask_needs_command" -f -a "complete-tasks" -d 'Internal helper for shell completions'
"#;

const FISH_COMPLETE_TASKS_SUBCOMMAND: &str = r#"complete -c rhask -n "__fish_rhask_using_subcommand complete-tasks" -s f -l file -d 'Path to the Rhai script file (defaults to searching for rhaskfile.rhai)' -r
complete -c rhask -n "__fish_rhask_using_subcommand complete-tasks" -s h -l help -d 'Print help'
"#;

const FISH_DYNAMIC_COMPLETIONS: &str = r#"

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
                # ignore
                ;
            case '*'
                # already have positional task argument
                return 1
        end
        set idx (math $idx + 1)
    end
    return 0
end

complete -c rhask -n "__fish_rhask_should_complete_tasks_direct" -a "(__fish_rhask_task_candidates)" -f
complete -c rhask -n "__fish_rhask_should_complete_tasks_run" -a "(__fish_rhask_task_candidates)" -f
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_task_name_returns_runtime_error() {
        let err = missing_task_name_error();
        match err.as_ref() {
            EvalAltResult::ErrorRuntime(msg, _) => {
                let text = msg.to_string();
                assert!(text.contains("Task name is required"));
            }
            other => panic!("expected runtime error, got {:?}", other),
        }
    }

    #[test]
    fn dispatcher_errors_for_direct_without_task_name() {
        let engine = engine::ScriptEngine::new();
        let result = dispatcher(Some(cli::Commands::Direct(vec![])), engine);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(format!("{}", err).contains("Task name is required"));
    }

    #[test]
    fn dispatcher_handles_list_command_without_panic() {
        let engine = engine::ScriptEngine::new();
        let opts = cli::ListOptions {
            group: Some("nonexistent".to_string()),
            flat: true,
        };
        let result = dispatcher(Some(cli::Commands::List(opts)), engine);
        assert!(result.is_ok());
    }

    #[test]
    fn dispatcher_lists_when_no_command_and_no_default() {
        let engine = engine::ScriptEngine::new();
        let result = dispatcher(None, engine);
        assert!(result.is_ok());
    }
}
