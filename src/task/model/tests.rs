use super::{BuildStack, TaskRegistry};
use crate::task::TaskLookup;
use rhai::{Dynamic, FnPtr, Map};

fn dummy_fn_ptr() -> FnPtr {
    FnPtr::new("dummy").unwrap()
}

#[test]
fn task_registration_and_context() {
    let mut registry = TaskRegistry::new();
    let mut stack = BuildStack::new();

    stack.begin_task(&registry, "build").unwrap();

    stack.set_description("desc").unwrap();
    stack.set_actions(dummy_fn_ptr()).unwrap();

    stack.end_task(&mut registry).unwrap();

    match registry.resolve_task("build") {
        TaskLookup::Found { full_path } => assert_eq!(full_path, "build"),
        other => panic!("task lookup failed: {:?}", other_desc(other)),
    }
}

#[test]
fn group_and_nested_task() {
    let mut registry = TaskRegistry::new();
    let mut stack = BuildStack::new();
    stack.begin_group(&registry, "build_suite").unwrap();
    stack.set_description("suite").unwrap();

    stack.begin_task(&registry, "build").unwrap();
    stack.end_task(&mut registry).unwrap();

    stack.begin_group(&registry, "release_flow").unwrap();
    stack.begin_task(&registry, "deploy").unwrap();
    stack.end_task(&mut registry).unwrap();
    stack.end_group(&mut registry).unwrap();

    stack.end_group(&mut registry).unwrap();

    assert!(matches!(
        registry.resolve_task("build_suite.build"),
        TaskLookup::Found { .. }
    ));
    assert!(matches!(
        registry.resolve_task("build_suite.release_flow.deploy"),
        TaskLookup::Found { .. }
    ));
}

#[test]
fn reject_task_redefinition() {
    let mut registry = TaskRegistry::new();
    let mut stack = BuildStack::new();
    stack.begin_task(&registry, "build").unwrap();
    stack.end_task(&mut registry).unwrap();

    let err = stack.begin_task(&registry, "build").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("is already defined"));
}

#[test]
fn reject_group_redefinition_as_task() {
    let mut registry = TaskRegistry::new();
    let mut stack = BuildStack::new();
    stack.begin_group(&registry, "ops").unwrap();
    stack.end_group(&mut registry).unwrap();

    let err = stack.begin_task(&registry, "ops").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("is already defined as a group"));
}

#[test]
fn nested_task_rejected() {
    let registry = TaskRegistry::new();
    let mut stack = BuildStack::new();
    stack.begin_task(&registry, "outer").unwrap();
    let err = stack.begin_task(&registry, "inner").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("Nested task() calls are not supported"));
}

#[test]
fn description_outside_context_fails() {
    let mut stack = BuildStack::new();
    let err = stack.set_description("no context").unwrap_err();
    assert!(err
        .to_string()
        .contains("description() can only be used inside task() or group()."));
}

#[test]
fn args_outside_task_fails() {
    let mut stack = BuildStack::new();
    let mut params = Map::new();
    params.insert("profile".into(), Dynamic::from("release"));

    let err = stack.set_args(params).unwrap_err();
    assert!(err
        .to_string()
        .contains("args() can only be used inside task()."));
}

fn other_desc(result: TaskLookup) -> String {
    match result {
        TaskLookup::Found { full_path } => format!("Found({})", full_path),
        TaskLookup::Ambiguous(paths) => format!("Ambiguous({paths:?})"),
        TaskLookup::NotFound => "NotFound".to_string(),
    }
}
