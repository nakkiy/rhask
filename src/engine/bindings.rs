use indexmap::IndexMap;
use rhai::{
    Array, Dynamic, Engine, EvalAltResult, FnPtr, ImmutableString, Map, NativeCallContext, Position,
};
use std::process::Command;
use std::sync::{Arc, Mutex};

use super::core::{actions_only_error, ActionScope, ExecutionState};
use crate::logger::{error, trace, warn};
use crate::printer;
use crate::task::{prepare_arguments_from_parts, TaskLookup, TaskRegistry};

type RegistryRef = Arc<Mutex<TaskRegistry>>;

pub fn register_all(
    engine: &mut Engine,
    registry: RegistryRef,
    exec_state: Arc<Mutex<ExecutionState>>,
) {
    register_contextual(
        engine,
        "task",
        registry.clone(),
        |registry, name| registry.begin_task(name),
        |registry| registry.end_task(),
    );

    register_contextual(
        engine,
        "group",
        registry.clone(),
        |registry, name| registry.begin_group(name),
        |registry| registry.end_group(),
    );

    let reg_clone = registry.clone();
    engine.register_fn(
        "actions",
        move |func: FnPtr| -> Result<(), Box<EvalAltResult>> {
            let mut reg = reg_clone.lock().unwrap();
            reg.set_actions(func)
        },
    );

    let reg_clone = registry.clone();
    engine.register_fn(
        "description",
        move |desc: &str| -> Result<(), Box<EvalAltResult>> {
            let mut reg = reg_clone.lock().unwrap();
            reg.set_description(desc)
        },
    );

    let reg_clone = registry.clone();
    engine.register_fn(
        "args",
        move |params: Map| -> Result<(), Box<EvalAltResult>> {
            let mut reg = reg_clone.lock().unwrap();
            reg.set_args(params)
        },
    );

    // Backward compatibility for the common typo
    let reg_clone = registry.clone();
    engine.register_fn(
        "discription",
        move |desc: &str| -> Result<(), Box<EvalAltResult>> {
            let mut reg = reg_clone.lock().unwrap();
            reg.set_description(desc)
        },
    );

    register_exec(engine, exec_state.clone());
    register_trigger(engine, registry, exec_state);
}

fn register_contextual<FStart, FEnd>(
    engine: &mut Engine,
    name: &str,
    registry: RegistryRef,
    start: FStart,
    end: FEnd,
) where
    FStart: Fn(&mut TaskRegistry, &str) -> Result<(), Box<EvalAltResult>> + Send + Sync + 'static,
    FEnd: Fn(&mut TaskRegistry) -> Result<(), Box<EvalAltResult>> + Send + Sync + 'static,
{
    engine.register_fn(
        name,
        move |ctx: NativeCallContext,
              identifier: &str,
              func: FnPtr|
              -> Result<(), Box<EvalAltResult>> {
            {
                let mut reg = registry.lock().unwrap();
                start(&mut reg, identifier)?;
            }

            let result = func.call_within_context::<()>(&ctx, ());

            {
                let mut reg = registry.lock().unwrap();
                end(&mut reg)?;
            }

            result
        },
    );
}

fn register_exec(engine: &mut Engine, state: Arc<Mutex<ExecutionState>>) {
    engine.register_fn(
        "exec",
        move |command: &str| -> Result<(), Box<EvalAltResult>> {
            {
                let guard = state.lock().unwrap();
                if !guard.is_active() {
                    return Err(actions_only_error("exec()"));
                }
            }

            let status = Command::new("sh")
                .arg("-c")
                .arg(command)
                .status()
                .map_err(|err| {
                    EvalAltResult::ErrorRuntime(
                        format!("Failed to execute command: {}", err).into(),
                        Position::NONE,
                    )
                })?;

            if status.success() {
                Ok(())
            } else {
                Err(EvalAltResult::ErrorRuntime(
                    format!("Command exited with failure (status: {})", status).into(),
                    Position::NONE,
                )
                .into())
            }
        },
    );
}

fn trigger_impl(
    ctx: &NativeCallContext,
    registry: &RegistryRef,
    state: &Arc<Mutex<ExecutionState>>,
    name: &str,
    positional: Vec<String>,
    named: IndexMap<String, String>,
) -> Result<(), Box<EvalAltResult>> {
    trace!(
        "trigger_impl called: name='{}', positional={:?}, named={:?}",
        name,
        positional,
        named
    );
    let lookup = {
        let reg = registry.lock().unwrap();
        reg.resolve_task(name)
    };

    match lookup {
        TaskLookup::Found { full_path } => {
            let (func, args) = {
                let reg = registry.lock().unwrap();
                let args = prepare_arguments_from_parts(&reg, &full_path, positional, named)?;
                let action = reg
                    .tasks
                    .get(&full_path)
                    .and_then(|task| task.actions.clone());
                (action, args)
            };

            {
                let guard = state.lock().unwrap();
                if !guard.is_active() {
                    error!(
                        "trigger_impl rejected: '{}' called outside actions context",
                        name
                    );
                    return Err(actions_only_error("trigger()"));
                }
            }

            if let Some(func) = func {
                trace!(
                    "trigger_impl executing '{}' with {} argument(s)",
                    full_path,
                    args.len()
                );
                let _scope = ActionScope::start_nested(state.clone(), "trigger()")?;
                call_with_context(ctx, &func, args)?;
            } else {
                warn!(
                    "trigger_impl: task '{}' has no actions registered",
                    full_path
                );
                printer::warn(format!("Task '{}' has no actions() registered.", full_path));
            }
            Ok(())
        }
        TaskLookup::Ambiguous(candidates) => {
            warn!(
                "trigger_impl: name '{}' ambiguous -> {:?}",
                name, candidates
            );
            printer::warn(format!("Task '{}' matches multiple candidates:", name));
            for candidate in candidates {
                printer::warn(format!("  - {}", candidate));
            }
            printer::warn("Please use the fully-qualified name (e.g. group.task).");
            Ok(())
        }
        TaskLookup::NotFound => {
            warn!("trigger_impl: task '{}' not found", name);
            printer::warn(format!("Task '{}' does not exist.", name));
            Ok(())
        }
    }
}

fn call_with_context(
    ctx: &NativeCallContext,
    func: &FnPtr,
    args: Vec<Dynamic>,
) -> Result<(), Box<EvalAltResult>> {
    let _ = func.call_within_context::<Dynamic>(ctx, args)?;
    Ok(())
}

fn array_to_positional(array: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    array
        .into_iter()
        .map(|value| Ok(value.to_string()))
        .collect()
}

fn map_to_named(map: Map) -> Result<IndexMap<String, String>, Box<EvalAltResult>> {
    let mut named = IndexMap::new();
    for (key, value) in map.into_iter() {
        let value = if value.is_unit() {
            String::new()
        } else if let Some(s) = value.clone().try_cast::<ImmutableString>() {
            s.into()
        } else {
            value.to_string()
        };
        named.insert(key.into(), value);
    }
    Ok(named)
}

fn register_trigger(engine: &mut Engine, registry: RegistryRef, state: Arc<Mutex<ExecutionState>>) {
    let reg_clone = registry.clone();
    let state_clone = state.clone();
    engine.register_fn(
        "trigger",
        move |ctx: NativeCallContext, name: &str| -> Result<(), Box<EvalAltResult>> {
            trigger_impl(
                &ctx,
                &reg_clone,
                &state_clone,
                name,
                Vec::new(),
                IndexMap::new(),
            )
        },
    );

    let reg_clone = registry.clone();
    let state_clone = state.clone();
    engine.register_fn(
        "trigger",
        move |ctx: NativeCallContext,
              name: &str,
              positional: Array|
              -> Result<(), Box<EvalAltResult>> {
            let positionals = array_to_positional(positional)?;
            trigger_impl(
                &ctx,
                &reg_clone,
                &state_clone,
                name,
                positionals,
                IndexMap::new(),
            )
        },
    );

    let reg_clone = registry.clone();
    let state_clone = state.clone();
    engine.register_fn(
        "trigger",
        move |ctx: NativeCallContext, name: &str, named: Map| -> Result<(), Box<EvalAltResult>> {
            let named_args = map_to_named(named)?;
            trigger_impl(&ctx, &reg_clone, &state_clone, name, Vec::new(), named_args)
        },
    );

    let reg_clone = registry;
    let state_clone = state;
    engine.register_fn(
        "trigger",
        move |ctx: NativeCallContext,
              name: &str,
              positional: Array,
              named: Map|
              -> Result<(), Box<EvalAltResult>> {
            let positionals = array_to_positional(positional)?;
            let named_args = map_to_named(named)?;
            trigger_impl(
                &ctx,
                &reg_clone,
                &state_clone,
                name,
                positionals,
                named_args,
            )
        },
    );
}
