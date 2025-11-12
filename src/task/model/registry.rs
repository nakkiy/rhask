use indexmap::IndexMap;

use super::types::{Group, RegistryEntry, Task};

#[derive(Clone)]
pub struct TaskRegistry {
    pub(super) tasks: IndexMap<String, Task>,
    pub(super) groups: IndexMap<String, Group>,
    pub(super) root_entries: Vec<RegistryEntry>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self {
            tasks: IndexMap::new(),
            groups: IndexMap::new(),
            root_entries: Vec::new(),
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
}

#[cfg(test)]
impl TaskRegistry {
    pub fn insert_task_for_test(&mut self, name: &str) {
        self.tasks.insert(name.to_string(), Task::default());
    }
}
