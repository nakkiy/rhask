mod cli;
mod engine;
mod logger;
mod printer;
mod task;

use crate::logger::*;
use rhai::EvalAltResult;

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
            engine.list_tasks(opts.group.as_deref());
            Ok(())
        }
        cli::Commands::Run(opts) => engine.run_task(&opts.task, &opts.args).map_err(|err| {
            error!("failed to execute command: {}", err);
            err
        }),
    }
}
