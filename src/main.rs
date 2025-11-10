mod cli;
mod engine;
mod logger;
mod printer;
mod task;

use crate::logger::*;
use rhai::{EvalAltResult, Position};

fn main() -> Result<(), Box<EvalAltResult>> {
    logger::init();
    info!("start");

    let cli = cli::parse_args();
    let mut engine = engine::ScriptEngine::new();
    let script_path = cli
        .file
        .clone()
        .unwrap_or_else(|| "rhaskfile.rhai".to_string());

    engine.run_script(&script_path)?;

    match dispatcher(cli.cmd, engine) {
        Ok(()) => {
            info!("{} end", env!("CARGO_PKG_NAME"));
            Ok(())
        }
        Err(_err) => {
            std::process::exit(1);
        }
    }
}

fn dispatcher(cmd: cli::Commands, engine: engine::ScriptEngine) -> Result<(), Box<EvalAltResult>> {
    match cmd {
        cli::Commands::List(opts) => {
            engine.list_tasks(opts.group.as_deref(), opts.flat);
            Ok(())
        }
        cli::Commands::Run(opts) => run_with_logging(engine, &opts.task, &opts.args),
        cli::Commands::Direct(raw) => {
            let (task, args) = raw.split_first().ok_or_else(missing_task_name_error)?;
            run_with_logging(engine, task, args)
        }
    }
}

fn run_with_logging(
    engine: engine::ScriptEngine,
    task: &str,
    args: &[String],
) -> Result<(), Box<EvalAltResult>> {
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
