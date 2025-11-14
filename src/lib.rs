#![doc = include_str!("../README.md")]

pub mod cli;
pub mod engine;
pub mod logger;
pub mod printer;
pub mod task;

use clap::Parser;
use cli::Cli;
use logger::*;
use rhai::{EvalAltResult, Position};

pub fn run() -> Result<(), Box<EvalAltResult>> {
    let cli = Cli::parse();
    run_with_cli(cli)
}

pub fn run_with_cli(cli: Cli) -> Result<(), Box<EvalAltResult>> {
    logger::init();
    info!("start");
    debug!("cli args: {:?}", cli);

    let mut script_engine = engine::ScriptEngine::new();
    let script_path = cli
        .file
        .clone()
        .unwrap_or_else(|| "rhaskfile.rhai".to_string());

    script_engine.run_script(&script_path)?;
    dispatcher(cli.cmd, script_engine)?;
    info!("{} end", env!("CARGO_PKG_NAME"));
    Ok(())
}

fn dispatcher(cmd: cli::Commands, engine: engine::ScriptEngine) -> Result<(), Box<EvalAltResult>> {
    debug!("dispatching command: {:?}", cmd);
    match cmd {
        cli::Commands::List(opts) => {
            info!("Listing tasks: group={:?}, flat={}", opts.group, opts.flat);
            engine.list_tasks(opts.group.as_deref(), opts.flat);
            Ok(())
        }
        cli::Commands::Run(opts) => run_with_logging(engine, &opts.task, &opts.args),
        cli::Commands::Direct(raw) => {
            let (task, args) = raw.split_first().ok_or_else(|| {
                warn!("Direct command invoked without a task name");
                missing_task_name_error()
            })?;
            run_with_logging(engine, task, args)
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
        let result = dispatcher(cli::Commands::Direct(vec![]), engine);
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
        let result = dispatcher(cli::Commands::List(opts), engine);
        assert!(result.is_ok());
    }
}
