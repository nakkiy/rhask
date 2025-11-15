const DYNAMIC_COMPLETIONS: &str = r#"

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

const REREGISTER: &str = r#"
if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _rhask -o nosort -o bashdefault -o default rhask
else
    complete -F _rhask -o bashdefault -o default rhask
fi
"#;

pub fn patch(mut script: String) -> String {
    if script.contains("_rhask()") {
        script = script.replacen("_rhask()", "__rhask_base()", 1);
        script.push_str(DYNAMIC_COMPLETIONS);
        script.push_str(REREGISTER);
    }
    script
}
