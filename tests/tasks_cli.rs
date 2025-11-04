use assert_cmd::Command;
use predicates::{prelude::PredicateBooleanExt, str::contains};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

fn rhask() -> Command {
    Command::cargo_bin("rhask").expect("rhask binary build failed")
}

fn fixture_rhaskfile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rhaskfile.rhai")
}

fn invalid_rhaskfile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rhaskfile_invalid_exec.rhai")
}

fn invalid_trigger_rhaskfile() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rhaskfile_invalid_trigger.rhai")
}

#[test]
fn list_prints_groups_and_tasks() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "list"])
        .assert()
        .success()
        .stdout(contains("> build_suite").and(contains("- build_debug")));
}

#[test]
fn list_specific_group_only() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "list", "build_suite"])
        .assert()
        .success()
        .stdout(contains("- build_debug").and(contains("> release_flow")))
        .stdout(contains("ops_suite").not());
}

#[test]
fn list_child_group_by_leaf_name() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "list",
            "build_suite.release_flow",
        ])
        .assert()
        .success()
        .stdout(contains("package_artifacts").and(contains("deploy_staging")))
        .stdout(contains("build_debug").not());
}

#[test]
fn run_unique_task_by_leaf_name() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "clean"])
        .assert()
        .success()
        .stdout(contains("Cleanup completed"));
}

#[test]
fn run_requires_full_path_when_ambiguous() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "deploy_staging"])
        .assert()
        .success()
        .stderr(contains("matches multiple candidates"));
}

#[test]
fn run_with_full_path_executes_task() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "build_suite.release_flow.deploy_staging",
        ])
        .assert()
        .success()
        .stdout(contains("deploy to staging (fixture)"));
}

#[test]
fn run_task_with_positional_args() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "build", "release"])
        .assert()
        .success()
        .stdout(contains("profile:release").and(contains("target:x86_64-unknown-linux-gnu")));
}

#[test]
fn run_task_with_named_args() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "build",
            "--target=x86_64-pc-windows-gnu",
        ])
        .assert()
        .success()
        .stdout(contains("target:x86_64-pc-windows-gnu").and(contains("profile:debug")));
}

#[test]
fn run_task_with_mixed_args() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "build",
            "release",
            "--target=x86_64-pc-windows-gnu",
        ])
        .assert()
        .success()
        .stdout(contains("profile:release").and(contains("target:x86_64-pc-windows-gnu")));
}

#[test]
fn run_task_with_unknown_arg_fails() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "build",
            "--unknown=value",
        ])
        .assert()
        .failure()
        .stderr(contains("Unknown argument"));
}

#[test]
fn run_task_missing_required_arg_fails() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "requires_version"])
        .assert()
        .failure()
        .stderr(contains("Argument 'version' is missing"));
}

#[test]
fn run_task_required_arg_success() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "requires_version",
            "v1.2.3",
        ])
        .assert()
        .success()
        .stdout(contains("requires_version => v1.2.3"));
}

#[test]
fn run_task_via_trigger_helper() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "trigger_clean"])
        .assert()
        .success()
        .stdout(contains("Cleanup completed"));
}

#[test]
fn trigger_with_array_arguments() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args([
            "--file",
            fixture_str.as_str(),
            "run",
            "trigger_build_release",
        ])
        .assert()
        .success()
        .stdout(contains("profile:release"));
}

#[test]
fn trigger_with_named_arguments() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "trigger_build_named"])
        .assert()
        .success()
        .stdout(contains("target:wasm32-unknown-unknown"));
}

#[test]
fn trigger_with_mixed_arguments() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "trigger_build_mixed"])
        .assert()
        .success()
        .stdout(contains("profile:release").and(contains("target:x86_64-apple-darwin")));
}

#[test]
fn run_exec_helper() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "run", "exec_echo"])
        .assert()
        .success()
        .stdout(contains("exec-from-fixture"));
}

#[test]
fn run_from_nested_directory_finds_rhaskfile() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let script_path = root.join("rhaskfile.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("hello", || {{
                description("temp task");
                actions(|| {{
                    print("Hello from temp");
                }});
            }});
        "#
    )
    .expect("write script");

    let nested = root.join("subdir/child");
    fs::create_dir_all(&nested).expect("create nested dirs");

    rhask()
        .args(["run", "hello"])
        .current_dir(&nested)
        .assert()
        .success()
        .stdout(contains("Hello from temp"));
}

#[test]
fn run_with_explicit_file_option() {
    let temp = tempdir().expect("create temp dir");
    let script_path = temp.path().join("custom.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("greet", || {{
                description("custom task");
                actions(|| {{
                    print("hi from custom file");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .args([
            "-f",
            script_path.to_str().expect("utf8 path"),
            "run",
            "greet",
        ])
        .assert()
        .success()
        .stdout(contains("hi from custom file"));
}

#[test]
fn exec_outside_actions_fails_on_load() {
    rhask()
        .args(["--file", invalid_rhaskfile().to_str().unwrap(), "list"])
        .assert()
        .failure()
        .stderr(contains("can only be used inside actions()."));
}

#[test]
fn trigger_outside_actions_fails_on_load() {
    rhask()
        .args([
            "--file",
            invalid_trigger_rhaskfile().to_str().unwrap(),
            "list",
        ])
        .assert()
        .failure()
        .stderr(contains("trigger() can only be used inside actions()."));
}

#[test]
fn list_unknown_group() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "list", "unknown_group"])
        .assert()
        .success()
        .stderr(contains("does not exist"));
}

#[test]
fn list_ambiguous_group() {
    let fixture = fixture_rhaskfile();
    let fixture_str = fixture.to_str().expect("utf8 fixture path").to_string();
    rhask()
        .args(["--file", fixture_str.as_str(), "list", "release_flow"])
        .assert()
        .success()
        .stderr(contains("matches multiple candidates"));
}
