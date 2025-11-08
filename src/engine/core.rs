use rhai::{Dynamic, Engine, EvalAltResult, FnPtr, Position, AST};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::bindings;
use crate::logger::*;
use crate::printer;
use crate::task::{prepare_arguments_from_cli, ListRenderMode, TaskLookup, TaskRegistry};

pub struct ScriptEngine {
    pub engine: Engine,
    pub registry: Arc<Mutex<TaskRegistry>>,
    pub ast: Option<AST>,
    pub(crate) exec_state: Arc<Mutex<ExecutionState>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Rhai's register_fn requires closures to be Send + Sync, so we use Arc<Mutex<_>>
        // even though access is effectively single-threaded.
        #[allow(clippy::arc_with_non_send_sync)]
        let registry = Arc::new(Mutex::new(TaskRegistry::new()));
        let exec_state = Arc::new(Mutex::new(ExecutionState::default()));

        engine.set_max_expr_depths(256, 128);

        bindings::register_all(&mut engine, registry.clone(), exec_state.clone());

        Self {
            engine,
            registry,
            ast: None,
            exec_state,
        }
    }

    pub fn run_script(&mut self, path: &str) -> Result<(), Box<EvalAltResult>> {
        let script_path = resolve_script_path(path).map_err(|err| -> Box<EvalAltResult> {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("Unable to locate script file '{}': {}", path, err).into(),
                Position::NONE,
            ))
        })?;

        debug!("run_script({})", script_path.display());

        let ast = self.engine.compile_file(script_path)?;
        trace!("run_script: AST compiled successfully");
        self.engine.run_ast(&ast)?;
        trace!("run_script: AST executed successfully");
        self.ast = Some(ast);
        Ok(())
    }

    pub fn list_tasks(&self, group: Option<&str>, flat: bool) {
        let mode = if flat {
            ListRenderMode::Flat
        } else {
            ListRenderMode::Tree
        };
        self.registry.lock().unwrap().list(group, mode);
    }

    pub fn run_task(&self, name: &str, raw_args: &[String]) -> Result<(), Box<EvalAltResult>> {
        debug!("run_task({})", name);

        let lookup = {
            let reg = self.registry.lock().unwrap();
            reg.resolve_task(name)
        };

        let (full_path, call_args, func) = match lookup {
            TaskLookup::Found { full_path } => {
                let (call_args, actions) = {
                    let reg = self.registry.lock().unwrap();
                    let args = prepare_arguments_from_cli(&reg, &full_path, raw_args)?;
                    trace!(
                        "run_task: resolved '{}' -> '{}', args_len={}, raw_args={:?}",
                        name,
                        full_path,
                        args.len(),
                        raw_args
                    );
                    let task_actions = reg
                        .tasks
                        .get(&full_path)
                        .and_then(|task| task.actions.clone());
                    (args, task_actions)
                };
                (full_path, call_args, actions)
            }
            TaskLookup::NotFound => {
                warn!("run_task: '{}' not found", name);
                printer::warn(format!("Task '{}' does not exist.", name));
                return Ok(());
            }
            TaskLookup::Ambiguous(candidates) => {
                warn!("run_task: '{}' ambiguous matches {:?}", name, candidates);
                printer::warn(format!("Task '{}' matches multiple candidates:", name));
                for candidate in candidates {
                    printer::warn(format!("  - {}", candidate));
                }
                printer::warn("Please use the fully-qualified name (e.g. group.task).");
                return Ok(());
            }
        };

        if let Some(ast) = &self.ast {
            if let Some(func) = func {
                let _scope = ActionScope::start(self.exec_state.clone());
                trace!(
                    "run_task: invoking actions for '{}' with {} argument(s)",
                    full_path,
                    call_args.len()
                );
                self.invoke_action(ast, func, call_args)?;
            } else {
                warn!("run_task: '{}' has no actions registered", full_path);
                printer::warn(format!("Task '{}' has no actions() registered.", full_path));
            }
        } else {
            error!("run_task: AST not loaded before executing '{}'", full_path);
            printer::error("AST is not loaded. Run the script first.");
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct ExecutionState {
    depth: usize,
}

impl ExecutionState {
    fn enter(&mut self) {
        self.depth += 1;
    }

    fn exit(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.depth > 0
    }
}

pub(crate) struct ActionScope {
    state: Arc<Mutex<ExecutionState>>,
}

impl ActionScope {
    pub(crate) fn start(state: Arc<Mutex<ExecutionState>>) -> Self {
        let mut guard = state.lock().unwrap();
        guard.enter();
        drop(guard);
        Self { state }
    }

    pub(crate) fn start_nested(
        state: Arc<Mutex<ExecutionState>>,
        label: &str,
    ) -> Result<Self, Box<EvalAltResult>> {
        let mut guard = state.lock().unwrap();
        if !guard.is_active() {
            return Err(actions_only_error(label));
        }
        guard.enter();
        drop(guard);
        Ok(Self { state })
    }
}

impl Drop for ActionScope {
    fn drop(&mut self) {
        let mut guard = self.state.lock().unwrap();
        guard.exit();
    }
}

pub(crate) fn actions_only_error(label: &str) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(
        format!("{} can only be used inside actions().", label).into(),
        Position::NONE,
    )
    .into()
}

impl ScriptEngine {
    fn invoke_action(
        &self,
        ast: &AST,
        func: FnPtr,
        args: Vec<Dynamic>,
    ) -> Result<(), Box<EvalAltResult>> {
        let _ = func.call::<Dynamic>(&self.engine, ast, args)?;
        Ok(())
    }
}

fn resolve_script_path(path: &str) -> io::Result<PathBuf> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        if candidate.exists() {
            return Ok(candidate.to_path_buf());
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Absolute path '{}' does not exist", candidate.display()),
            ));
        }
    }

    let mut current = env::current_dir()?;
    loop {
        let joined = current.join(path);
        if joined.exists() {
            return Ok(joined);
        }
        if !current.pop() {
            break;
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "Could not find '{}' when walking up parent directories",
            path
        ),
    ))
}
