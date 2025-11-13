use indexmap::IndexMap;
use rhai::{Dynamic, EvalAltResult, ImmutableString};

use crate::logger::{debug, trace};

use super::model::context_error;
use super::registry::TaskRegistry;

#[cfg(test)]
use super::stack::BuildStack;

#[allow(clippy::type_complexity)]
pub fn parse_cli_arguments(
    raw_args: &[String],
) -> Result<(Vec<String>, IndexMap<String, String>), Box<EvalAltResult>> {
    trace!("parse_cli_arguments: raw_args={:?}", raw_args);
    let mut positional = Vec::new();
    let mut named = IndexMap::new();
    let mut i = 0;
    while i < raw_args.len() {
        let arg = &raw_args[i];
        if let Some(rest) = arg.strip_prefix("--") {
            if rest.is_empty() {
                return Err(context_error("Argument name required after '--'."));
            }
            let (key, value) = if let Some((key, value)) = rest.split_once('=') {
                (key.to_string(), value.to_string())
            } else if i + 1 < raw_args.len()
                && !raw_args[i + 1].starts_with("--")
                && !raw_args[i + 1].contains('=')
            {
                i += 1;
                (rest.to_string(), raw_args[i].clone())
            } else {
                return Err(context_error(format!(
                    "Option '--{}' is missing a value.",
                    rest
                )));
            };
            if key.is_empty() {
                return Err(context_error("Argument name cannot be empty."));
            }
            named.insert(key, value);
        } else if let Some((key, value)) = arg.split_once('=') {
            if key.is_empty() {
                return Err(context_error("Argument name cannot be empty."));
            }
            named.insert(key.to_string(), value.to_string());
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }
    trace!(
        "parse_cli_arguments result -> positional={:?}, named={:?}",
        positional,
        named
    );
    Ok((positional, named))
}

pub fn prepare_arguments_from_cli(
    registry: &TaskRegistry,
    task_name: &str,
    raw_args: &[String],
) -> Result<Vec<Dynamic>, Box<EvalAltResult>> {
    trace!(
        "prepare_arguments_from_cli: task='{}', raw_args={:?}",
        task_name,
        raw_args
    );
    let (positional, named) = parse_cli_arguments(raw_args)?;
    prepare_arguments_from_parts(registry, task_name, positional, named)
}

pub fn prepare_arguments_from_parts(
    registry: &TaskRegistry,
    task_name: &str,
    positional: Vec<String>,
    named: IndexMap<String, String>,
) -> Result<Vec<Dynamic>, Box<EvalAltResult>> {
    debug!(
        "prepare_arguments_from_parts: task='{}', positional={:?}, named={:?}",
        task_name, positional, named
    );
    prepare_arguments_internal(registry, task_name, positional, named)
}

fn prepare_arguments_internal(
    registry: &TaskRegistry,
    task_name: &str,
    positional: Vec<String>,
    mut named: IndexMap<String, String>,
) -> Result<Vec<Dynamic>, Box<EvalAltResult>> {
    let task = registry
        .task(task_name)
        .ok_or_else(|| context_error(format!("Internal error: task '{}' not found", task_name)))?;

    if task.params.is_empty() {
        if positional.is_empty() && named.is_empty() {
            trace!(
                "task '{}' expects no args and none were provided",
                task_name
            );
            return Ok(Vec::new());
        } else {
            return Err(context_error(format!(
                "Task '{}' does not accept arguments.",
                task_name
            )));
        }
    }

    let mut values = Vec::with_capacity(task.params.len());
    let mut positional_iter = positional.into_iter();

    for spec in &task.params {
        if let Some(value) = named.shift_remove(&spec.name) {
            values.push(Dynamic::from(ImmutableString::from(value)));
        } else if let Some(value) = positional_iter.next() {
            values.push(Dynamic::from(ImmutableString::from(value)));
        } else if let Some(default) = &spec.default {
            values.push(Dynamic::from(ImmutableString::from(default.clone())));
        } else {
            return Err(context_error(format!(
                "Argument '{}' is missing.",
                spec.name
            )));
        }
    }

    if let Some(extra) = positional_iter.next() {
        return Err(context_error(format!(
            "Unexpected positional argument '{}' provided.",
            extra
        )));
    }

    if !named.is_empty() {
        let unknown: Vec<String> = named.keys().cloned().collect();
        return Err(context_error(format!(
            "Unknown argument(s): {}",
            unknown.join(", ")
        )));
    }

    trace!("prepared arguments for '{}': {:?}", task_name, values);
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Dynamic, Map};

    fn registry_with_args() -> TaskRegistry {
        let mut registry = TaskRegistry::new();
        let mut stack = BuildStack::new();
        stack.begin_task(&registry, "build").unwrap();
        let mut params = Map::new();
        params.insert("profile".into(), Dynamic::from("debug"));
        params.insert("target".into(), Dynamic::from("x86"));
        stack.set_args(params).unwrap();
        stack.end_task(&mut registry).unwrap();
        registry
    }

    #[test]
    fn parse_cli_arguments_supports_mixed_forms() {
        let raw = vec![
            "release".to_string(),
            "--target=x86".to_string(),
            "profile=debug".to_string(),
            "--arch".to_string(),
            "arm64".to_string(),
        ];

        let (positional, named) = parse_cli_arguments(&raw).expect("parse args");

        assert_eq!(positional, vec!["release"]);
        assert_eq!(named.get("target"), Some(&"x86".to_string()));
        assert_eq!(named.get("profile"), Some(&"debug".to_string()));
        assert_eq!(named.get("arch"), Some(&"arm64".to_string()));
    }

    #[test]
    fn prepare_arguments_prioritizes_named_over_positional() {
        let registry = registry_with_args();

        let args = prepare_arguments_from_cli(
            &registry,
            "build",
            &[
                "release".to_string(),
                "--target=wasm32-unknown-unknown".to_string(),
            ],
        )
        .expect("prepare args");

        let collected: Vec<String> = args
            .into_iter()
            .map(|value| value.into_string().expect("string"))
            .collect();
        assert_eq!(
            collected,
            vec!["release".to_string(), "wasm32-unknown-unknown".to_string()]
        );
    }

    #[test]
    fn prepare_arguments_reports_unknown_keys() {
        let registry = registry_with_args();

        let err = prepare_arguments_from_cli(&registry, "build", &["--unknown=value".to_string()])
            .unwrap_err();
        assert!(err.to_string().contains("Unknown argument(s):"));
    }
}
