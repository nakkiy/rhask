use indexmap::IndexMap;
use rhai::{plugin::*, Array, Engine, EvalAltResult, FnPtr, Map, NativeCallContext, Position};
use std::process::Command;

use super::runtime::{
    array_to_positional, ensure_actions_scope, map_to_named, runtime_from_ctx, trigger_impl,
    with_build_stack, ScopeGuard, ScopeKind,
};
use crate::logger::{debug, error, trace};

pub fn register(engine: &mut Engine) {
    engine.register_global_module(exported_module!(rhask_api).into());
}

#[export_module]
pub mod rhask_api {
    use super::*;

    #[rhai_fn(global, name = "task", return_raw)]
    pub fn task_block(
        ctx: NativeCallContext,
        identifier: &str,
        func: FnPtr,
    ) -> Result<(), Box<EvalAltResult>> {
        let _guard = ScopeGuard::enter(&ctx, identifier, ScopeKind::Task)?;
        func.call_within_context::<()>(&ctx, ())
    }

    #[rhai_fn(global, name = "group", return_raw)]
    pub fn group_block(
        ctx: NativeCallContext,
        identifier: &str,
        func: FnPtr,
    ) -> Result<(), Box<EvalAltResult>> {
        let _guard = ScopeGuard::enter(&ctx, identifier, ScopeKind::Group)?;
        func.call_within_context::<()>(&ctx, ())
    }

    #[rhai_fn(global, name = "actions", return_raw)]
    pub fn register_actions(ctx: NativeCallContext, func: FnPtr) -> Result<(), Box<EvalAltResult>> {
        with_build_stack(&ctx, move |stack| stack.set_actions(func))
    }

    #[rhai_fn(global, name = "description", return_raw)]
    pub fn set_description(ctx: NativeCallContext, desc: &str) -> Result<(), Box<EvalAltResult>> {
        with_build_stack(&ctx, |stack| stack.set_description(desc))
    }

    #[rhai_fn(global, name = "discription", return_raw)]
    pub fn set_discription(ctx: NativeCallContext, desc: &str) -> Result<(), Box<EvalAltResult>> {
        set_description(ctx, desc)
    }

    #[rhai_fn(global, name = "args", return_raw)]
    pub fn set_args(ctx: NativeCallContext, params: Map) -> Result<(), Box<EvalAltResult>> {
        with_build_stack(&ctx, move |stack| stack.set_args(params))
    }

    #[rhai_fn(global, name = "exec", return_raw)]
    pub fn exec_command(ctx: NativeCallContext, command: &str) -> Result<(), Box<EvalAltResult>> {
        let runtime = runtime_from_ctx(&ctx)?;
        ensure_actions_scope(&runtime.exec_state, "exec()")?;
        trace!("exec() invoked with command: {}", command);

        let status = Command::new("sh")
            .arg("-c")
            .arg(command)
            .status()
            .map_err(|err| {
                error!("Failed to spawn command '{}': {}", command, err);
                EvalAltResult::ErrorRuntime(
                    format!("Failed to execute command: {}", err).into(),
                    Position::NONE,
                )
            })?;

        if status.success() {
            debug!(
                "Command '{}' finished successfully (status: {:?})",
                command,
                status.code()
            );
            Ok(())
        } else {
            error!(
                "Command '{}' exited with failure (status: {:?})",
                command,
                status.code()
            );
            Err(EvalAltResult::ErrorRuntime(
                format!("Command exited with failure (status: {})", status).into(),
                Position::NONE,
            )
            .into())
        }
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn trigger_simple(ctx: NativeCallContext, name: &str) -> Result<(), Box<EvalAltResult>> {
        trigger_impl(&ctx, name, Vec::new(), IndexMap::new())
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn trigger_with_positional(
        ctx: NativeCallContext,
        name: &str,
        positional: Array,
    ) -> Result<(), Box<EvalAltResult>> {
        let positionals = array_to_positional(positional)?;
        trigger_impl(&ctx, name, positionals, IndexMap::new())
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn trigger_with_named(
        ctx: NativeCallContext,
        name: &str,
        named: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let named_args = map_to_named(named)?;
        trigger_impl(&ctx, name, Vec::new(), named_args)
    }

    #[rhai_fn(global, name = "trigger", return_raw)]
    pub fn trigger_with_both(
        ctx: NativeCallContext,
        name: &str,
        positional: Array,
        named: Map,
    ) -> Result<(), Box<EvalAltResult>> {
        let positionals = array_to_positional(positional)?;
        let named_args = map_to_named(named)?;
        trigger_impl(&ctx, name, positionals, named_args)
    }
}
