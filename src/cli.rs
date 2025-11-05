use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rhask",
    version = env!("CARGO_PKG_VERSION"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    long_about = None
)]
pub struct Cli {
    /// Path to the Rhai script file (defaults to searching for rhaskfile.rhai)
    #[arg(short, long, value_name = "FILE", global = true)]
    pub file: Option<String>,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show the task list (`rhask list -h` for details)
    List(ListOptions),
    /// Run a task (`rhask run -h` for details)
    Run(RunOptions),
}

#[derive(Args, Debug)]
pub struct ListOptions {
    /// Group name to display (omit to show every task)
    #[arg(name = "GROUP")]
    pub group: Option<String>,
}

#[derive(Args, Debug)]
#[command(trailing_var_arg = true)]
pub struct RunOptions {
    /// Task name to execute
    #[arg(name = "TASK_NAME")]
    pub task: String,

    /// Arguments passed to the task
    #[arg(name = "ARGS", allow_hyphen_values = true)]
    pub args: Vec<String>,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
