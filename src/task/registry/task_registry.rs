use indexmap::IndexMap;
use rhai::EvalAltResult;

use crate::task::model::{context_error, Group, RegistryEntry, Task};

#[derive(Clone)]
pub struct TaskRegistry {
    tasks: IndexMap<String, Task>,
    groups: IndexMap<String, Group>,
    root_entries: Vec<RegistryEntry>,
    default_task: Option<String>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self {
            tasks: IndexMap::new(),
            groups: IndexMap::new(),
            root_entries: Vec::new(),
            default_task: None,
        }
    }
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn task(&self, path: &str) -> Option<&Task> {
        self.tasks.get(path)
    }

    pub(crate) fn contains_task(&self, path: &str) -> bool {
        self.tasks.contains_key(path)
    }

    pub(crate) fn group(&self, path: &str) -> Option<&Group> {
        self.groups.get(path)
    }

    pub(crate) fn contains_group(&self, path: &str) -> bool {
        self.groups.contains_key(path)
    }

    pub(crate) fn root_entries(&self) -> &[RegistryEntry] {
        &self.root_entries
    }

    pub(crate) fn tasks_iter(&self) -> impl Iterator<Item = (&String, &Task)> {
        self.tasks.iter()
    }

    pub(crate) fn groups_iter(&self) -> impl Iterator<Item = (&String, &Group)> {
        self.groups.iter()
    }

    pub(crate) fn insert_task_entry(&mut self, full_path: String, task: Task) {
        self.tasks.insert(full_path, task);
    }

    pub(crate) fn insert_group_entry(&mut self, full_path: String, group: Group) {
        self.groups.insert(full_path, group);
    }

    pub(crate) fn push_root_entry(&mut self, entry: RegistryEntry) {
        self.root_entries.push(entry);
    }

    pub(crate) fn set_default_task(&mut self, name: &str) -> Result<(), Box<EvalAltResult>> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(context_error("default_task() requires a task name."));
        }
        if self.default_task.is_some() {
            return Err(context_error(
                "default_task() can only be defined once per rhaskfile.",
            ));
        }
        self.default_task = Some(trimmed.to_string());
        Ok(())
    }

    pub(crate) fn default_task(&self) -> Option<String> {
        self.default_task.clone()
    }
}

#[cfg(test)]
impl TaskRegistry {
    pub fn insert_task_for_test(&mut self, name: &str) {
        self.insert_task_entry(name.to_string(), Task::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_task_rejects_empty_or_whitespace() {
        let mut registry = TaskRegistry::new();
        assert!(registry.set_default_task("").is_err());
        assert!(registry.set_default_task("   ").is_err());
        assert!(registry.default_task().is_none());
    }

    #[test]
    fn default_task_allows_single_definition() {
        let mut registry = TaskRegistry::new();
        registry
            .set_default_task(" build.clean ")
            .expect("set default");
        assert_eq!(registry.default_task().as_deref(), Some("build.clean"));
        assert!(registry.set_default_task("other").is_err());
    }
}
