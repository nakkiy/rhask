#![doc = include_str!("../README.md")]

pub mod cli;
pub mod completions;
pub mod engine;
pub mod logger;
pub mod printer;
pub mod task;

pub use completions::print as print_shell_completions;

use clap::Parser;
use cli::Cli;
use logger::*;
use rhai::{EvalAltResult, Position};
use std::io::{self, Write};

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

fn print_task_candidates(engine: &engine::ScriptEngine, prefix: &str) {
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

    let mut stdout = io::BufWriter::new(io::stdout());
    for name in entries.into_iter().filter(|name| name.starts_with(prefix)) {
        let _ = writeln!(stdout, "{name}");
    }
    let _ = stdout.flush();
}

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
