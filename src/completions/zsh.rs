const REGISTRATION: &str = r#"if [ "$funcstack[1]" = "_rhask" ]; then
    _rhask "$@"
else
    compdef _rhask rhask
fi
"#;

const DYNAMIC_COMPLETIONS: &str = r#"

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
                local -a dynamic described
                local candidate
                local __rhask_prev_insert __rhask_prev_list
                dynamic=( ${(f)"$(__rhask_dynamic_tasks "$cur")"} )
                if (( ${#dynamic[@]} )); then
                    described=()
                    for candidate in "${dynamic[@]}"; do
                        described+=("$candidate:task/group")
                    done
                    if (( ${#dynamic[@]} > 1 )); then
                        __rhask_prev_insert="$compstate[insert]"
                        __rhask_prev_list="$compstate[list]"
                        compstate[insert]=''
                        compstate[list]='list'
                        _describe -t rhask-tasks 'task or group' described && handled=1
                        compstate[insert]="$__rhask_prev_insert"
                        compstate[list]="$__rhask_prev_list"
                    else
                        _describe -t rhask-tasks 'task or group' described && handled=1
                    fi
                fi
                ;;
        esac
    elif (( first_idx > 0 )) && [[ "${words[first_idx]}" = run ]] && (( CURRENT == first_idx + 1 )); then
        local -a dynamic described
        local candidate
        local __rhask_prev_insert __rhask_prev_list
        dynamic=( ${(f)"$(__rhask_dynamic_tasks "$cur")"} )
        if (( ${#dynamic[@]} )); then
            described=()
            for candidate in "${dynamic[@]}"; do
                described+=("$candidate:task/group")
            done
            if (( ${#dynamic[@]} > 1 )); then
                __rhask_prev_insert="$compstate[insert]"
                __rhask_prev_list="$compstate[list]"
                compstate[insert]=''
                compstate[list]='list'
                _describe -t rhask-run-tasks 'task or group' described && handled=1
                compstate[insert]="$__rhask_prev_insert"
                compstate[list]="$__rhask_prev_list"
            else
                _describe -t rhask-run-tasks 'task or group' described && handled=1
            fi
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

pub fn patch(mut script: String) -> String {
    if script.contains("_rhask()") {
        script = script.replacen("_rhask()", "__rhask_base()", 1);
        script = script.replace(REGISTRATION, "");
        script.push_str(DYNAMIC_COMPLETIONS);
    }
    script
}
