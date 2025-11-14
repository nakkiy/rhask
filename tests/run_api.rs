use rhask::cli::{Cli, Commands, ListOptions};
use rhask::run_with_cli;

fn fixture_rhaskfile() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rhaskfile.rhai");
    path.to_str().expect("utf8 fixture path").to_string()
}

#[test]
fn run_with_cli_lists_tasks_successfully() {
    let cli = Cli {
        file: Some(fixture_rhaskfile()),
        cmd: Some(Commands::List(ListOptions {
            group: None,
            flat: false,
        })),
    };

    run_with_cli(cli).expect("list command should succeed");
}

#[test]
fn run_with_cli_propagates_errors() {
    let cli = Cli {
        file: Some(fixture_rhaskfile()),
        cmd: Some(Commands::Direct(Vec::new())),
    };

    assert!(run_with_cli(cli).is_err());
}
