use super::task_registry::TaskRegistry;
use crate::logger::trace;
use crate::task::model::leaf_name;

pub enum TaskLookup {
    Found { full_path: String },
    Ambiguous(Vec<String>),
    NotFound,
}

impl TaskRegistry {
    pub fn resolve_task(&self, identifier: &str) -> TaskLookup {
        let trimmed = identifier.trim();
        trace!("resolve_task: identifier='{}'", identifier);
        if trimmed.is_empty() {
            return TaskLookup::NotFound;
        }

        if self.contains_task(trimmed) {
            trace!("resolve_task: '{}' matched exact task", trimmed);
            return TaskLookup::Found {
                full_path: trimmed.to_string(),
            };
        }

        if trimmed.contains('.') {
            if self.contains_task(trimmed) {
                trace!("resolve_task: '{}' matched dotted task", trimmed);
                return TaskLookup::Found {
                    full_path: trimmed.to_string(),
                };
            } else {
                trace!("resolve_task: dotted identifier '{}' not found", trimmed);
                return TaskLookup::NotFound;
            }
        }

        let matches: Vec<String> = self
            .tasks_iter()
            .filter(|(full_path, _)| leaf_name(full_path) == trimmed)
            .map(|(full_path, _)| full_path.clone())
            .collect();

        match matches.len() {
            0 => {
                trace!("resolve_task: '{}' not found as leaf", trimmed);
                TaskLookup::NotFound
            }
            1 => {
                let full_path = matches.into_iter().next().unwrap();
                trace!(
                    "resolve_task: leaf '{}' resolved uniquely to '{}'",
                    trimmed,
                    full_path
                );
                TaskLookup::Found { full_path }
            }
            _ => {
                trace!(
                    "resolve_task: leaf '{}' ambiguous matches {:?}",
                    trimmed,
                    matches
                );
                TaskLookup::Ambiguous(matches)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_tasks(names: &[&str]) -> TaskRegistry {
        let mut registry = TaskRegistry::new();
        for name in names {
            registry.insert_task_for_test(name);
        }
        registry
    }

    #[test]
    fn resolves_full_path() {
        let registry = registry_with_tasks(&["ops.deploy"]);
        match registry.resolve_task("ops.deploy") {
            TaskLookup::Found { full_path } => assert_eq!(full_path, "ops.deploy"),
            other => panic!("unexpected lookup result: {:?}", result_desc(other)),
        }
    }

    #[test]
    fn resolves_leaf_when_unique() {
        let registry = registry_with_tasks(&["ops.deploy"]);
        match registry.resolve_task("deploy") {
            TaskLookup::Found { full_path } => assert_eq!(full_path, "ops.deploy"),
            other => panic!("unexpected lookup result: {:?}", result_desc(other)),
        }
    }

    #[test]
    fn detects_ambiguous_leaf() {
        let registry = registry_with_tasks(&["ops.deploy", "build.release.deploy"]);
        match registry.resolve_task("deploy") {
            TaskLookup::Ambiguous(mut paths) => {
                paths.sort();
                assert_eq!(
                    paths,
                    vec!["build.release.deploy".to_string(), "ops.deploy".to_string()]
                );
            }
            other => panic!("expected ambiguous, got {:?}", result_desc(other)),
        }
    }

    #[test]
    fn trims_identifier() {
        let registry = registry_with_tasks(&["ops.deploy"]);
        match registry.resolve_task("  ops.deploy  ") {
            TaskLookup::Found { full_path } => assert_eq!(full_path, "ops.deploy"),
            other => panic!("unexpected lookup result: {:?}", result_desc(other)),
        }
    }

    #[test]
    fn not_found_when_empty_or_missing() {
        let registry = registry_with_tasks(&["ops.deploy"]);
        assert!(matches!(registry.resolve_task(""), TaskLookup::NotFound));
        assert!(matches!(
            registry.resolve_task("unknown"),
            TaskLookup::NotFound
        ));
    }

    fn result_desc(result: TaskLookup) -> String {
        match result {
            TaskLookup::Found { full_path } => format!("Found({})", full_path),
            TaskLookup::Ambiguous(paths) => format!("Ambiguous({paths:?})"),
            TaskLookup::NotFound => "NotFound".to_string(),
        }
    }
}
