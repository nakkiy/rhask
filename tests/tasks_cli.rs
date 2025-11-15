use assert_cmd::Command;
use predicates::{
    prelude::PredicateBooleanExt,
    str::{contains, is_match},
};
use std::{fs, io::Write};
use tempfile::tempdir;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rhaskfile.rhai");
const INVALID_EXEC: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/rhaskfile_invalid_exec.rhai"
);
const INVALID_TRIGGER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/rhaskfile_invalid_trigger.rhai"
);
const INVALID_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/rhaskfile_invalid_dir.rhai"
);

fn rhask() -> Command {
    Command::cargo_bin("rhask").expect("rhask binary build failed")
}

fn rhask_with_fixture() -> Command {
    let mut cmd = rhask();
    cmd.args(["--file", FIXTURE]);
    cmd
}

fn build_log(profile: &str, target: &str) -> String {
    format!("[build] profile={}, target={}", profile, target)
}

const CLEAN_LOG: &str = "[clean] workspace cleaned";
const TRIGGER_CLEAN_LOG: &str = "[trigger_clean] delegating to clean";
const REQUIRE_VERSION_PREFIX: &str = "[requires_version] version=";

#[test]
fn list_prints_groups_and_tasks() {
    rhask_with_fixture()
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("> build_suite").and(contains("- clean")));
}

#[test]
fn list_flat_prints_full_paths() {
    rhask_with_fixture()
        .args(["list", "--flat"])
        .assert()
        .success()
        .stdout(
            is_match(r"build_suite\.release_flow\.deploy_staging\s+Deploy to staging")
                .expect("regex compile"),
        )
        .stdout(is_match(r"clean\s+Remove build artifacts").expect("regex compile"));
}

#[test]
fn list_specific_group_only() {
    rhask_with_fixture()
        .args(["list", "build_suite"])
        .assert()
        .success()
        .stdout(contains("- build_debug").and(contains("> release_flow")))
        .stdout(contains("ops_suite").not());
}

#[test]
fn list_child_group_by_leaf_name() {
    rhask_with_fixture()
        .args(["list", "build_suite.release_flow"])
        .assert()
        .success()
        .stdout(contains("package_artifacts").and(contains("deploy_staging")))
        .stdout(contains("build_debug").not());
}

#[test]
fn list_unknown_group() {
    rhask_with_fixture()
        .args(["list", "unknown_group"])
        .assert()
        .success()
        .stderr(contains("Group 'unknown_group' does not exist."));
}

#[test]
fn list_ambiguous_group() {
    rhask_with_fixture()
        .args(["list", "release_flow"])
        .assert()
        .success()
        .stderr(contains("matches multiple candidates"));
}

#[test]
fn run_unique_task_by_leaf_name() {
    rhask_with_fixture()
        .args(["run", "clean"])
        .assert()
        .success()
        .stdout(contains(CLEAN_LOG));
}

#[test]
fn run_without_command_executes_default_task() {
    rhask_with_fixture()
        .assert()
        .success()
        .stdout(contains(CLEAN_LOG));
}

#[test]
fn shorthand_run_executes_task() {
    rhask_with_fixture()
        .args(["clean"])
        .assert()
        .success()
        .stdout(contains(CLEAN_LOG));
}

#[test]
fn shorthand_run_accepts_arguments() {
    rhask_with_fixture()
        .args(["build", "release", "--target=x86_64-pc-windows-gnu"])
        .assert()
        .success()
        .stdout(contains(build_log("release", "x86_64-pc-windows-gnu")));
}

#[test]
fn run_requires_full_path_when_ambiguous() {
    rhask_with_fixture()
        .args(["run", "deploy_staging"])
        .assert()
        .failure()
        .stderr(contains(
            "error: Task 'deploy_staging' matches multiple candidates:",
        ));
}

#[test]
fn run_with_full_path_executes_task() {
    rhask_with_fixture()
        .args(["run", "build_suite.release_flow.deploy_staging"])
        .assert()
        .success()
        .stdout(contains("[deploy_staging] deploy to staging (fixture)"));
}

#[test]
fn run_task_with_positional_args() {
    rhask_with_fixture()
        .args(["run", "build", "release"])
        .assert()
        .success()
        .stdout(contains(build_log("release", "x86_64-unknown-linux-gnu")));
}

#[test]
fn run_task_with_named_args() {
    rhask_with_fixture()
        .args(["run", "build", "--target=x86_64-pc-windows-gnu"])
        .assert()
        .success()
        .stdout(contains(build_log("debug", "x86_64-pc-windows-gnu")));
}

#[test]
fn run_task_with_mixed_args() {
    rhask_with_fixture()
        .args(["run", "build", "release", "--target=x86_64-pc-windows-gnu"])
        .assert()
        .success()
        .stdout(contains(build_log("release", "x86_64-pc-windows-gnu")));
}

#[test]
fn run_task_with_unknown_arg_fails() {
    rhask_with_fixture()
        .args(["run", "build", "--unknown=value"])
        .assert()
        .failure()
        .stderr(contains("Unknown argument"));
}

#[test]
fn run_task_missing_required_arg_fails() {
    rhask_with_fixture()
        .args(["run", "requires_version"])
        .assert()
        .failure()
        .stderr(contains("Argument 'version' is missing"));
}

#[test]
fn run_task_required_arg_success() {
    rhask_with_fixture()
        .args(["run", "requires_version", "v1.2.3"])
        .assert()
        .success()
        .stdout(contains(format!("{REQUIRE_VERSION_PREFIX}v1.2.3")));
}

#[test]
fn run_task_via_trigger_helper() {
    rhask_with_fixture()
        .args(["run", "trigger_clean"])
        .assert()
        .success()
        .stdout(contains(TRIGGER_CLEAN_LOG).and(contains(CLEAN_LOG)));
}

#[test]
fn trigger_with_array_arguments() {
    rhask_with_fixture()
        .args(["run", "trigger_build_release"])
        .assert()
        .success()
        .stdout(contains(build_log("release", "x86_64-unknown-linux-gnu")));
}

#[test]
fn trigger_with_named_arguments() {
    rhask_with_fixture()
        .args(["run", "trigger_build_named"])
        .assert()
        .success()
        .stdout(contains(build_log("debug", "wasm32-unknown-unknown")));
}

#[test]
fn trigger_with_mixed_arguments() {
    rhask_with_fixture()
        .args(["run", "trigger_build_mixed"])
        .assert()
        .success()
        .stdout(contains(build_log("release", "x86_64-apple-darwin")));
}

#[test]
fn run_exec_helper() {
    rhask_with_fixture()
        .args(["run", "exec_echo"])
        .assert()
        .success()
        .stdout(contains("[exec_echo]"))
        .stdout(contains("fixture-exec-output"));
}

#[test]
fn run_no_actions_fails() {
    rhask_with_fixture()
        .args(["run", "no_actions"])
        .assert()
        .failure()
        .stderr(contains("has no actions() registered."));
}

#[test]
fn run_trigger_unknown_fails() {
    rhask_with_fixture()
        .args(["run", "trigger_unknown"])
        .assert()
        .failure()
        .stderr(contains("Task 'unknown_task_for_tests' does not exist."));
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
                    print("[hello] temp");
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
        .stdout(contains("[hello] temp"));
}

#[test]
fn dir_exec_uses_rhaskfile_root_for_relative_paths() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts).expect("create scripts dir");

    let script_path = root.join("rhaskfile.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("write_pwd", || {{
                dir("scripts");
                actions(|| {{
                    exec("pwd > cwd.txt");
                }});
            }});
        "#
    )
    .expect("write script");

    let nested = root.join("subdir/child");
    fs::create_dir_all(&nested).expect("create nested dirs");

    rhask()
        .current_dir(&nested)
        .args(["run", "write_pwd"])
        .assert()
        .success();

    let cwd_file = scripts.join("cwd.txt");
    let contents = fs::read_to_string(&cwd_file).expect("read cwd output");
    let expected = scripts
        .canonicalize()
        .expect("canonicalize scripts dir")
        .to_str()
        .expect("utf8 path")
        .to_string();
    assert_eq!(contents.trim(), expected);
}

#[test]
fn dir_settings_do_not_leak_between_triggered_tasks() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let parent_dir = root.join("parent_dir");
    let child_dir = root.join("child_dir");
    fs::create_dir_all(&parent_dir).expect("create parent dir");
    fs::create_dir_all(&child_dir).expect("create child dir");

    let script_path = root.join("rhaskfile.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("child_task", || {{
                dir("child_dir");
                actions(|| {{
                    exec("pwd > child_cwd.txt");
                }});
            }});

            task("parent_task", || {{
                dir("parent_dir");
                actions(|| {{
                    exec("pwd > parent_cwd.txt");
                    trigger("child_task");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .current_dir(root)
        .args(["run", "parent_task"])
        .assert()
        .success();

    let parent_contents =
        fs::read_to_string(parent_dir.join("parent_cwd.txt")).expect("read parent cwd");
    let child_contents =
        fs::read_to_string(child_dir.join("child_cwd.txt")).expect("read child cwd");

    let expected_parent = parent_dir
        .canonicalize()
        .expect("canonical parent dir")
        .to_str()
        .expect("utf8 path")
        .to_string();
    let expected_child = child_dir
        .canonicalize()
        .expect("canonical child dir")
        .to_str()
        .expect("utf8 path")
        .to_string();

    assert_eq!(parent_contents.trim(), expected_parent);
    assert_eq!(child_contents.trim(), expected_child);
}

#[test]
fn child_without_dir_runs_in_launcher_directory() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let parent_dir = root.join("parent_dir");
    fs::create_dir_all(&parent_dir).expect("create parent dir");

    let script_path = root.join("rhaskfile.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("child_task", || {{
                actions(|| {{
                    exec("pwd > child_cwd.txt");
                }});
            }});

            task("parent_task", || {{
                dir("parent_dir");
                actions(|| {{
                    trigger("child_task");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .current_dir(root)
        .args(["run", "parent_task"])
        .assert()
        .success();

    let child_file = root.join("child_cwd.txt");
    assert!(
        child_file.exists(),
        "child_cwd.txt should be created at the launch directory"
    );
    let contents = fs::read_to_string(&child_file).expect("read child cwd");
    let expected = root
        .canonicalize()
        .expect("canonicalize root dir")
        .to_str()
        .expect("utf8 path")
        .to_string();
    assert_eq!(contents.trim(), expected);
    assert!(
        !parent_dir.join("child_cwd.txt").exists(),
        "child file must not be written under the parent's dir"
    );
}

#[test]
fn child_dir_applies_even_when_parent_has_none() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let child_dir = root.join("child_dir");
    fs::create_dir_all(&child_dir).expect("create child dir");

    let script_path = root.join("rhaskfile.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("child_task", || {{
                dir("child_dir");
                actions(|| {{
                    exec("pwd > child_cwd.txt");
                }});
            }});

            task("parent_task", || {{
                actions(|| {{
                    exec("pwd > parent_cwd.txt");
                    trigger("child_task");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .current_dir(root)
        .args(["run", "parent_task"])
        .assert()
        .success();

    let parent_contents = fs::read_to_string(root.join("parent_cwd.txt")).expect("read parent cwd");
    let child_contents =
        fs::read_to_string(child_dir.join("child_cwd.txt")).expect("read child cwd");

    let expected_parent = root
        .canonicalize()
        .expect("canonical root dir")
        .to_str()
        .expect("utf8 path")
        .to_string();
    let expected_child = child_dir
        .canonicalize()
        .expect("canonical child dir")
        .to_str()
        .expect("utf8 path")
        .to_string();

    assert_eq!(parent_contents.trim(), expected_parent);
    assert_eq!(child_contents.trim(), expected_child);
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
                    print("[greet] hi from custom file");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .args([
            "--file",
            script_path.to_str().expect("utf8 path"),
            "run",
            "greet",
        ])
        .assert()
        .success()
        .stdout(contains("[greet] hi from custom file"));
}

#[test]
fn run_without_command_lists_when_default_missing() {
    let temp = tempdir().expect("create temp dir");
    let script_path = temp.path().join("nodefault.rhai");
    let mut file = fs::File::create(&script_path).expect("create script file");
    writeln!(
        file,
        r#"
            task("hello", || {{
                actions(|| {{
                    print("[hello] hi");
                }});
            }});
        "#
    )
    .expect("write script");

    rhask()
        .args(["--file", script_path.to_str().expect("utf8 path")])
        .assert()
        .success()
        .stdout(contains("- hello"));
}

#[test]
fn exec_outside_actions_fails_on_load() {
    rhask()
        .args(["--file", INVALID_EXEC, "list"])
        .assert()
        .failure()
        .stderr(contains("can only be used inside actions()."));
}

#[test]
fn trigger_outside_actions_fails_on_load() {
    rhask()
        .args(["--file", INVALID_TRIGGER, "list"])
        .assert()
        .failure()
        .stderr(contains("trigger() can only be used inside actions()."));
}

#[test]
fn complete_tasks_lists_candidates() {
    rhask()
        .args(["--file", FIXTURE, "complete-tasks", "build"])
        .assert()
        .success()
        .stdout(contains("build_suite").and(contains("build_suite.release_flow")));
}

#[test]
fn generate_completions_outputs_script() {
    rhask()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(contains("complete -F").and(contains("rhask")));
}

#[test]
fn generate_zsh_completions_include_dynamic_tasks() {
    rhask()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(
            contains("__rhask_dynamic_tasks")
                .and(contains("dynamic=( ${(f)\"$(__rhask_dynamic_tasks \"$cur\")\"} )"))
                .and(contains("compstate[insert]=''"))
                .and(contains("_describe -t rhask-tasks 'task or group' described"))
                .and(contains("compdef _rhask rhask")),
        );
}

#[test]
fn generate_fish_completions_include_dynamic_tasks() {
    rhask()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(
            contains("__fish_rhask_task_candidates")
                .and(contains("__fish_rhask_should_complete_tasks_run")),
        );
}

#[test]
fn dir_defined_twice_in_same_task_fails_on_load() {
    rhask()
        .args(["--file", INVALID_DIR, "list"])
        .assert()
        .failure()
        .stderr(contains("dir() can only be defined once per task()."));
}
