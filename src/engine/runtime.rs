use indexmap::IndexMap;
use rhai::{
    Array, Dynamic, EvalAltResult, FnPtr, ImmutableString, Map, NativeCallContext, Position,
};
use std::sync::{Arc, Mutex};

use super::core::{actions_only_error, ActionScope, ExecutionState};
use crate::logger::{error, trace, warn};
use crate::printer;
use crate::task::{prepare_arguments_from_parts, TaskLookup, TaskRegistry};

pub(super) type RegistryRef = Arc<Mutex<TaskRegistry>>;

#[derive(Clone)]
pub(super) struct RuntimeHandle {
    pub(super) registry: RegistryRef,
    pub(super) exec_state: Arc<Mutex<ExecutionState>>,
}

impl RuntimeHandle {
    pub(super) fn new(registry: RegistryRef, exec_state: Arc<Mutex<ExecutionState>>) -> Self {
        Self {
            registry,
            exec_state,
        }
    }
}

pub(super) fn trigger_impl(
    ctx: &NativeCallContext,
    name: &str,
    positional: Vec<String>,
    named: IndexMap<String, String>,
) -> Result<(), Box<EvalAltResult>> {
    let runtime = runtime_from_ctx(ctx)?;
    let registry = &runtime.registry;
    let state = &runtime.exec_state;
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

pub(super) fn array_to_positional(array: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    array
        .into_iter()
        .map(|value| Ok(value.to_string()))
        .collect()
}

pub(super) fn map_to_named(map: Map) -> Result<IndexMap<String, String>, Box<EvalAltResult>> {
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

pub(super) fn runtime_from_ctx(
    ctx: &NativeCallContext,
) -> Result<RuntimeHandle, Box<EvalAltResult>> {
    ctx.tag()
        .and_then(|tag| tag.read_lock::<RuntimeHandle>())
        .map(|handle| handle.clone())
        .ok_or_else(|| {
            EvalAltResult::ErrorRuntime(
                "Rhask runtime context is not available.".into(),
                Position::NONE,
            )
            .into()
        })
}

pub(super) fn ensure_actions_scope(
    state: &Arc<Mutex<ExecutionState>>,
    label: &str,
) -> Result<(), Box<EvalAltResult>> {
    let guard = state.lock().unwrap();
    if !guard.is_active() {
        return Err(actions_only_error(label));
    }
    Ok(())
}

pub(super) fn with_registry<F, R>(ctx: &NativeCallContext, op: F) -> Result<R, Box<EvalAltResult>>
where
    F: FnOnce(&mut TaskRegistry) -> Result<R, Box<EvalAltResult>>,
{
    let runtime = runtime_from_ctx(ctx)?;
    let mut registry = runtime.registry.lock().unwrap();
    op(&mut registry)
}

pub(super) fn contextual_block<FStart, FEnd>(
    ctx: &NativeCallContext,
    identifier: &str,
    func: FnPtr,
    start: FStart,
    end: FEnd,
) -> Result<(), Box<EvalAltResult>>
where
    FStart: Fn(&mut TaskRegistry, &str) -> Result<(), Box<EvalAltResult>>,
    FEnd: Fn(&mut TaskRegistry) -> Result<(), Box<EvalAltResult>>,
{
    with_registry(ctx, |registry| start(registry, identifier))?;
    let result = func.call_within_context::<()>(ctx, ());
    with_registry(ctx, |registry| end(registry))?;
    result
}
