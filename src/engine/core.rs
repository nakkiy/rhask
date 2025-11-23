use rhai::{Dynamic, Engine, EvalAltResult, FnPtr, Position, AST};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::bindings;
use crate::logger::*;
use crate::task::{
    prepare_arguments_from_cli, BuildStack, ListRenderMode, TaskLookup, TaskRegistry,
};

pub struct ScriptEngine {
    pub engine: Engine,
    pub registry: Arc<Mutex<TaskRegistry>>,
    pub ast: Option<AST>,
    pub(crate) exec_state: Arc<Mutex<ExecutionState>>,
    pub(crate) build_stack: Arc<Mutex<BuildStack>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Rhai's register_fn requires closures to be Send + Sync, so we use Arc<Mutex<_>>
        // even though access is effectively single-threaded.
        #[allow(clippy::arc_with_non_send_sync)]
        let registry = Arc::new(Mutex::new(TaskRegistry::new()));
        let exec_state = Arc::new(Mutex::new(ExecutionState::new()));
        #[allow(clippy::arc_with_non_send_sync)]
        let build_stack = Arc::new(Mutex::new(BuildStack::new()));

        engine.set_max_expr_depths(256, 128);

        bindings::register_all(
            &mut engine,
            registry.clone(),
            exec_state.clone(),
            build_stack.clone(),
        );

        Self {
            engine,
            registry,
            ast: None,
            exec_state,
            build_stack,
        }
    }

    pub fn run_script(&mut self, path: &str) -> Result<(), Box<EvalAltResult>> {
        let script_path = resolve_script_path(path).map_err(|err| -> Box<EvalAltResult> {
            Box::new(EvalAltResult::ErrorRuntime(
                format!("Unable to locate script file '{}': {}", path, err).into(),
                Position::NONE,
            ))
        })?;
        {
            let mut stack = self.build_stack.lock().unwrap();
            stack.reset();
            let parent = script_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            stack.set_script_root(parent);
        }

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

        let (full_path, call_args, func, task_dir) = match lookup {
            TaskLookup::Found { full_path } => {
                let (call_args, actions, working_dir) = {
                    let reg = self.registry.lock().unwrap();
                    let args = prepare_arguments_from_cli(&reg, &full_path, raw_args)?;
                    trace!(
                        "run_task: resolved '{}' -> '{}', args_len={}, raw_args={:?}",
                        name,
                        full_path,
                        args.len(),
                        raw_args
                    );
                    let task_meta = reg.task(&full_path);
                    let task_actions = task_meta.and_then(|task| task.actions.clone());
                    let working_dir = task_meta.and_then(|task| task.working_dir.clone());
                    (args, task_actions, working_dir)
                };
                (full_path, call_args, actions, working_dir)
            }
            TaskLookup::NotFound => {
                warn!("run_task: '{}' not found", name);
                return Err(user_error(format!("Task '{}' does not exist.", name)));
            }
            TaskLookup::Ambiguous(candidates) => {
                warn!("run_task: '{}' ambiguous matches {:?}", name, candidates);
                let mut message = format!("Task '{}' matches multiple candidates:\n", name);
                for candidate in candidates {
                    message.push_str(&format!("  - {}\n", candidate));
                }
                message.push_str("Please use the fully-qualified name (e.g. group.task).");
                return Err(user_error(message));
            }
        };

        if let Some(ast) = &self.ast {
            if let Some(func) = func {
                let _scope = ActionScope::start(self.exec_state.clone(), task_dir)?;
                trace!(
                    "run_task: invoking actions for '{}' with {} argument(s)",
                    full_path,
                    call_args.len()
                );
                self.invoke_action(ast, func, call_args)?;
            } else {
                warn!("run_task: '{}' has no actions registered", full_path);
                return Err(user_error(format!(
                    "Task '{}' has no actions() registered.",
                    full_path
                )));
            }
        } else {
            error!("run_task: AST not loaded before executing '{}'", full_path);
            return Err(user_error("AST is not loaded. Run the script first."));
        }
        Ok(())
    }

    pub fn default_task(&self) -> Option<String> {
        self.registry
            .lock()
            .ok()
            .and_then(|registry| registry.default_task())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct ExecutionState {
    contexts: Vec<ActionContext>,
    base_dir: PathBuf,
}

impl ExecutionState {
    fn new() -> Self {
        let base_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            contexts: Vec::new(),
            base_dir,
        }
    }

    fn push(&mut self, working_dir: Option<PathBuf>) {
        self.contexts.push(ActionContext::new(working_dir));
    }

    fn pop(&mut self) {
        self.contexts.pop();
    }

    fn enter_directory(
        &self,
        working_dir: Option<&PathBuf>,
    ) -> Result<Option<PathBuf>, Box<EvalAltResult>> {
        let target_dir = working_dir
            .cloned()
            .unwrap_or_else(|| self.base_dir.clone());
        let previous_dir = env::current_dir().map_err(|err| {
            user_error(format!("Failed to read current directory: {}", err))
        })?;
        if previous_dir != target_dir {
            env::set_current_dir(&target_dir).map_err(|err| {
                user_error(format!(
                    "Failed to change working directory to '{}': {}",
                    target_dir.display(),
                    err
                ))
            })?;
        }
        Ok(Some(previous_dir))
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.contexts.is_empty()
    }

    pub(crate) fn current_dir(&self) -> Option<PathBuf> {
        self.contexts.last().and_then(|ctx| ctx.working_dir.clone())
    }
}

#[derive(Clone, Debug)]
struct ActionContext {
    working_dir: Option<PathBuf>,
}

impl ActionContext {
    fn new(working_dir: Option<PathBuf>) -> Self {
        Self { working_dir }
    }
}

pub(crate) struct ActionScope {
    state: Arc<Mutex<ExecutionState>>,
    previous_dir: Option<PathBuf>,
}

impl ActionScope {
    pub(crate) fn start(
        state: Arc<Mutex<ExecutionState>>,
        working_dir: Option<PathBuf>,
    ) -> Result<Self, Box<EvalAltResult>> {
        let previous_dir = {
            let mut guard = state.lock().unwrap();
            let previous = guard.enter_directory(working_dir.as_ref())?;
            guard.push(working_dir);
            previous
        };
        Ok(Self { state, previous_dir })
    }

    pub(crate) fn start_nested(
        state: Arc<Mutex<ExecutionState>>,
        label: &str,
        working_dir: Option<PathBuf>,
    ) -> Result<Self, Box<EvalAltResult>> {
        let previous_dir = {
            let mut guard = state.lock().unwrap();
            if !guard.is_active() {
                return Err(actions_only_error(label));
            }
            let previous = guard.enter_directory(working_dir.as_ref())?;
            guard.push(working_dir);
            previous
        };
        Ok(Self { state, previous_dir })
    }
}

impl Drop for ActionScope {
    fn drop(&mut self) {
        let mut guard = self.state.lock().unwrap();
        guard.pop();
        if let Some(prev) = self.previous_dir.take() {
            if let Err(err) = env::set_current_dir(&prev) {
                warn!("Failed to restore working directory: {}", err);
            }
        }
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

pub(crate) fn user_error(message: impl Into<String>) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(
        message.into().into(),
        Position::NONE,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_script(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("create temp Rhai script");
        write!(file, "{contents}").expect("write script");
        file
    }

    #[test]
    fn run_task_errors_when_task_missing() {
        let script = write_script(
            r#"
            task("hello", || {
                actions(|| { print("hi"); });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("unknown_task", &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("Task 'unknown_task' does not exist."),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn run_task_errors_when_name_ambiguous() {
        let script = write_script(
            r#"
            group("build", || {
                task("deploy", || {
                    actions(|| { print("build deploy"); });
                });
            });
            group("ops", || {
                task("deploy", || {
                    actions(|| { print("ops deploy"); });
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("deploy", &[]).unwrap_err();
        assert!(
            err.to_string().contains("matches multiple candidates"),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn run_task_errors_when_actions_missing() {
        let script = write_script(
            r#"
            task("no_actions", || {
                description("intentionally missing actions()");
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("no_actions", &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("Task 'no_actions' has no actions() registered."),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn run_task_errors_when_ast_missing() {
        let script = write_script(
            r#"
            task("hello", || {
                actions(|| { print("hi"); });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        engine.ast = None;
        let err = engine.run_task("hello", &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("AST is not loaded. Run the script first."),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn trigger_errors_when_target_is_missing() {
        let script = write_script(
            r#"
            task("caller", || {
                actions(|| {
                    trigger("missing_task");
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("caller", &[]).unwrap_err();
        assert!(
            err.to_string().contains("does not exist"),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn trigger_errors_when_target_is_ambiguous() {
        let script = write_script(
            r#"
            group("build", || {
                task("deploy", || {
                    actions(|| {
                        print("build deploy");
                    });
                });
            });
            group("ops", || {
                task("deploy", || {
                    actions(|| {
                        print("ops deploy");
                    });
                });
            });
            task("caller", || {
                actions(|| {
                    trigger("deploy");
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("caller", &[]).unwrap_err();
        assert!(
            err.to_string().contains("matches multiple candidates"),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn trigger_errors_when_target_has_no_actions() {
        let script = write_script(
            r#"
            task("no_actions", || {
                description("intentionally empty");
            });
            task("caller", || {
                actions(|| {
                    trigger("no_actions");
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let err = engine.run_task("caller", &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("Task 'no_actions' has no actions() registered."),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn trigger_passes_positional_arguments() {
        let script = write_script(
            r#"
            task("target", || {
                args(#{ profile: "debug", arch: "x86_64" });
                actions(|arch, profile| {
                    if profile != "release" || arch != "arm64" {
                        throw "arguments not forwarded correctly";
                    }
                });
            });
            task("caller", || {
                actions(|| {
                    trigger("target", ["arm64", "release"]);
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        engine
            .run_task("caller", &[])
            .expect("trigger with positional args");
    }

    #[test]
    fn trigger_passes_named_arguments() {
        let script = write_script(
            r#"
            task("target", || {
                args(#{ profile: "debug", arch: "x86_64" });
                actions(|arch, profile| {
                    if profile != "release" || arch != "arm64" {
                        throw "named arguments not forwarded correctly";
                    }
                });
            });
            task("caller", || {
                actions(|| {
                    trigger("target", #{ arch: "arm64", profile: "release" });
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        engine
            .run_task("caller", &[])
            .expect("trigger with named args");
    }

    #[test]
    fn trigger_passes_mixed_arguments() {
        let script = write_script(
            r#"
            task("target", || {
                args(#{ profile: "debug", arch: "x86_64" });
                actions(|arch, profile| {
                    if profile != "release" || arch != "arm64" {
                        throw "mixed arguments not forwarded correctly";
                    }
                });
            });
            task("caller", || {
                actions(|| {
                    trigger("target", ["release"], #{ arch: "arm64" });
                });
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        engine
            .run_task("caller", &[])
            .expect("trigger with mixed args");
    }

    #[test]
    fn discription_alias_sets_description() {
        let script = write_script(
            r#"
            task("alias_desc", || {
                discription("legacy alias");
                actions(|| {});
            });
        "#,
        );
        let mut engine = ScriptEngine::new();
        engine
            .run_script(script.path().to_str().unwrap())
            .expect("load script");
        let registry = engine.registry.lock().unwrap();
        let task = registry
            .task("alias_desc")
            .expect("task registered via discription");
        assert_eq!(task.description.as_deref(), Some("legacy alias"));
    }
}
