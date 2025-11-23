use indexmap::IndexMap;
use rhai::{
    plugin::*, Array, Engine, EvalAltResult, FnPtr, ImmutableString, Map, NativeCallContext,
    Position,
};

use super::runtime::{
    array_to_positional, ensure_actions_scope, map_to_named, runtime_from_ctx, trigger_impl,
    with_build_stack, ScopeGuard, ScopeKind,
};
use rhai_process::PipelineExecutor;
use std::io::{self, Write};

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

    #[rhai_fn(global, name = "dir", return_raw)]
    pub fn set_directory(ctx: NativeCallContext, path: &str) -> Result<(), Box<EvalAltResult>> {
        with_build_stack(&ctx, |stack| stack.set_directory(path))
    }

    #[rhai_fn(global, name = "default_task", return_raw)]
    pub fn register_default_task(
        ctx: NativeCallContext,
        name: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let runtime = runtime_from_ctx(&ctx)?;
        let mut registry = runtime.registry.lock().unwrap();
        registry.set_default_task(name)
    }

    #[rhai_fn(global, name = "discription", return_raw)]
    pub fn set_discription(ctx: NativeCallContext, desc: &str) -> Result<(), Box<EvalAltResult>> {
        set_description(ctx, desc)
    }

    #[rhai_fn(global, name = "args", return_raw)]
    pub fn set_args(ctx: NativeCallContext, params: Map) -> Result<(), Box<EvalAltResult>> {
        with_build_stack(&ctx, move |stack| stack.set_args(params))
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

    #[rhai_fn(global, name = "exec", return_raw)]
    pub fn exec_executor(
        ctx: NativeCallContext,
        executor: PipelineExecutor,
    ) -> Result<Map, Box<EvalAltResult>> {
        run_executor(&ctx, executor, ExecMode::Run)
    }

    #[rhai_fn(global, name = "exec_stream", return_raw)]
    pub fn exec_stream_default(
        ctx: NativeCallContext,
        executor: PipelineExecutor,
    ) -> Result<Map, Box<EvalAltResult>> {
        run_executor(
            &ctx,
            executor,
            ExecMode::Stream {
                stdout: None,
                stderr: None,
            },
        )
    }

    #[rhai_fn(global, name = "exec_stream", return_raw)]
    pub fn exec_stream_stdout(
        ctx: NativeCallContext,
        executor: PipelineExecutor,
        stdout_cb: FnPtr,
    ) -> Result<Map, Box<EvalAltResult>> {
        run_executor(
            &ctx,
            executor,
            ExecMode::Stream {
                stdout: Some(stdout_cb),
                stderr: None,
            },
        )
    }

    #[rhai_fn(global, name = "exec_stream", return_raw)]
    pub fn exec_stream_both(
        ctx: NativeCallContext,
        executor: PipelineExecutor,
        stdout_cb: FnPtr,
        stderr_cb: FnPtr,
    ) -> Result<Map, Box<EvalAltResult>> {
        run_executor(
            &ctx,
            executor,
            ExecMode::Stream {
                stdout: Some(stdout_cb),
                stderr: Some(stderr_cb),
            },
        )
    }
}

enum ExecMode {
    Run,
    Stream {
        stdout: Option<FnPtr>,
        stderr: Option<FnPtr>,
    },
}

fn run_executor(
    ctx: &NativeCallContext,
    executor: PipelineExecutor,
    mode: ExecMode,
) -> Result<Map, Box<EvalAltResult>> {
    let runtime = runtime_from_ctx(ctx)?;
    ensure_actions_scope(&runtime.exec_state, "exec()")?;
    let executor = apply_working_dir(executor, &runtime)?;
    match mode {
        ExecMode::Run => {
            let result = executor.run()?;
            forward_streams(&result);
            Ok(result)
        }
        ExecMode::Stream { stdout, stderr } => executor.run_stream(ctx, stdout, stderr),
    }
}

fn apply_working_dir(
    executor: PipelineExecutor,
    runtime: &super::runtime::RuntimeHandle,
) -> Result<PipelineExecutor, Box<EvalAltResult>> {
    let working_dir = {
        let guard = runtime.exec_state.lock().unwrap();
        guard.current_dir()
    };
    if let Some(dir) = working_dir {
        let path = dir.to_str().map(|s| s.to_string()).ok_or_else(|| {
            EvalAltResult::ErrorRuntime(
                format!("dir(): '{}' cannot be represented as UTF-8", dir.display()).into(),
                Position::NONE,
            )
        })?;
        executor.cwd(path)
    } else {
        Ok(executor)
    }
}

fn forward_streams(result: &Map) {
    if let Some(stdout) = extract_string(result, "stdout") {
        if !stdout.is_empty() {
            print!("{}", stdout);
            let _ = io::stdout().flush();
        }
    }
    if let Some(stderr) = extract_string(result, "stderr") {
        if !stderr.is_empty() {
            eprint!("{}", stderr);
            let _ = io::stderr().flush();
        }
    }
}

fn extract_string(map: &Map, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|value| value.clone().try_cast::<ImmutableString>().map(Into::into))
}
