use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
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
    pub cmd: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show the task list (`rhask list -h` for details)
    List(ListOptions),
    /// Run a task (`rhask run -h` for details)
    Run(RunOptions),
    /// Execute a task directly (shorthand for `rhask run <task>`)
    #[command(external_subcommand)]
    Direct(Vec<String>),
}

#[derive(Args, Debug)]
pub struct ListOptions {
    /// Group name to display (omit to show every task)
    #[arg(name = "GROUP")]
    pub group: Option<String>,

    /// Print tasks as flat full paths (good for piping into fzf)
    #[arg(short = 'F', long = "flat")]
    pub flat: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_from<I, T>(items: I) -> Cli
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        Cli::parse_from(items)
    }

    #[test]
    fn parse_list_with_flat_option_and_group() {
        let cli = parse_from([
            "rhask",
            "-f",
            "custom.rhai",
            "list",
            "ops.release",
            "--flat",
        ]);
        assert_eq!(cli.file.as_deref(), Some("custom.rhai"));
        match cli.cmd.expect("list command") {
            Commands::List(opts) => {
                assert_eq!(opts.group.as_deref(), Some("ops.release"));
                assert!(opts.flat);
            }
            other => panic!("expected list command, got {:?}", other),
        }
    }

    #[test]
    fn parse_run_with_task_and_args() {
        let cli = parse_from([
            "rhask",
            "--file",
            "demo.rhai",
            "run",
            "build",
            "release",
            "--target=x86_64",
            "--arch",
            "arm64",
        ]);
        assert_eq!(cli.file.as_deref(), Some("demo.rhai"));
        match cli.cmd.expect("run command") {
            Commands::Run(opts) => {
                assert_eq!(opts.task, "build");
                assert_eq!(
                    opts.args,
                    vec![
                        "release".to_string(),
                        "--target=x86_64".to_string(),
                        "--arch".to_string(),
                        "arm64".to_string()
                    ]
                );
            }
            other => panic!("expected run command, got {:?}", other),
        }
    }

    #[test]
    fn parse_direct_subcommand_with_arguments() {
        let cli = parse_from(["rhask", "-f", "tasks.rhai", "deploy", "--env=prod", "extra"]);
        assert_eq!(cli.file.as_deref(), Some("tasks.rhai"));
        match cli.cmd.expect("direct command") {
            Commands::Direct(values) => {
                assert_eq!(
                    values,
                    vec![
                        "deploy".to_string(),
                        "--env=prod".to_string(),
                        "extra".to_string()
                    ]
                );
            }
            other => panic!("expected direct command, got {:?}", other),
        }
    }
}
