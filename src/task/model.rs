use indexmap::IndexMap;
use rhai::{EvalAltResult, FnPtr, ImmutableString, Map, Position};

#[derive(Clone, Debug)]
pub(crate) enum ContextKind {
    Root,
    Group { full_path: String },
    Task { full_path: String },
}

#[derive(Clone, Default)]
pub struct Task {
    pub description: Option<String>,
    pub actions: Option<FnPtr>,
    pub params: Vec<ParameterSpec>,
}

#[derive(Clone, Debug)]
pub struct ParameterSpec {
    pub name: String,
    pub default: Option<String>,
}

#[derive(Clone, Default)]
pub struct Group {
    pub description: Option<String>,
    pub entries: Vec<RegistryEntry>,
}

#[derive(Clone)]
pub enum RegistryEntry {
    Task(String),
    Group(String),
}

#[derive(Clone)]
pub struct TaskRegistry {
    pub(crate) tasks: IndexMap<String, Task>,
    pub(crate) groups: IndexMap<String, Group>,
    pub(crate) root_entries: Vec<RegistryEntry>,
    pub(crate) context_stack: Vec<ContextKind>,
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self {
            tasks: IndexMap::new(),
            groups: IndexMap::new(),
            root_entries: Vec::new(),
            context_stack: vec![ContextKind::Root],
        }
    }
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin_task(&mut self, name: &str) -> Result<(), Box<EvalAltResult>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(context_error("Task name cannot be empty."));
        }

        let full_path = self.build_child_path(name)?;

        if self.tasks.contains_key(&full_path) {
            return Err(context_error(format!(
                "Task '{}' is already defined.",
                full_path
            )));
        }

        if self.groups.contains_key(&full_path) {
            return Err(context_error(format!(
                "Task '{}' is already defined as a group.",
                full_path
            )));
        }

        self.push_entry_to_parent(RegistryEntry::Task(full_path.clone()))?;
        self.tasks.insert(full_path.clone(), Task::default());
        self.context_stack.push(ContextKind::Task { full_path });
        Ok(())
    }

    pub fn end_task(&mut self) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.pop() {
            Some(ContextKind::Task { .. }) => Ok(()),
            Some(ContextKind::Group { .. }) => Err(context_error(
                "context mismatch: end_task() called while inside group().",
            )),
            Some(ContextKind::Root) => Err(context_error(
                "context mismatch: end_task() called before task() was started.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub fn begin_group(&mut self, name: &str) -> Result<(), Box<EvalAltResult>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(context_error("Group name cannot be empty."));
        }

        let full_path = self.build_child_path(name)?;

        if self.groups.contains_key(&full_path) {
            return Err(context_error(format!(
                "Group '{}' is already defined.",
                full_path
            )));
        }

        if self.tasks.contains_key(&full_path) {
            return Err(context_error(format!(
                "Group '{}' is already defined as a task.",
                full_path
            )));
        }

        self.groups.insert(full_path.clone(), Group::default());
        self.push_entry_to_parent(RegistryEntry::Group(full_path.clone()))?;
        self.context_stack.push(ContextKind::Group { full_path });
        Ok(())
    }

    pub fn end_group(&mut self) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.pop() {
            Some(ContextKind::Group { .. }) => Ok(()),
            Some(ContextKind::Task { .. }) => Err(context_error(
                "context mismatch: task() scope was not closed before ending group().",
            )),
            Some(ContextKind::Root) => Err(context_error(
                "context mismatch: end_group() called before group() was started.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub fn set_actions(&mut self, func: FnPtr) -> Result<(), Box<EvalAltResult>> {
        let task = self.current_task_mut()?;
        task.actions = Some(func);
        Ok(())
    }

    pub fn set_args(&mut self, params: Map) -> Result<(), Box<EvalAltResult>> {
        let task = match self.context_stack.last().cloned() {
            Some(ContextKind::Task { full_path }) => self
                .tasks
                .get_mut(&full_path)
                .ok_or_else(|| context_error("Internal error: registered task not found"))?,
            Some(ContextKind::Group { .. }) | Some(ContextKind::Root) => {
                return Err(context_error("args() can only be used inside task()."));
            }
            None => {
                return Err(context_error("context mismatch: context stack is empty."));
            }
        };

        let mut entries: Vec<(String, Option<String>)> = params
            .into_iter()
            .map(|(key, value)| {
                let default = if value.is_unit() {
                    None
                } else if let Some(s) = value.clone().try_cast::<ImmutableString>() {
                    Some(s.into())
                } else {
                    Some(value.to_string())
                };
                (key.into(), default)
            })
            .collect();

        entries.sort_by(|a, b| a.0.cmp(&b.0));

        task.params = entries
            .into_iter()
            .map(|(name, default)| ParameterSpec { name, default })
            .collect();
        Ok(())
    }

    pub fn set_description(&mut self, desc: &str) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.last().cloned() {
            Some(ContextKind::Task { full_path }) => {
                let task = self
                    .tasks
                    .get_mut(&full_path)
                    .ok_or_else(|| context_error("Internal error: registered task not found"))?;
                task.description = Some(desc.to_string());
                Ok(())
            }
            Some(ContextKind::Group { full_path }) => {
                let group = self
                    .groups
                    .get_mut(&full_path)
                    .ok_or_else(|| context_error("Internal error: registered group not found"))?;
                group.description = Some(desc.to_string());
                Ok(())
            }
            Some(ContextKind::Root) => Err(context_error(
                "description() can only be used inside task() or group().",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub(crate) fn current_task_mut(&mut self) -> Result<&mut Task, Box<EvalAltResult>> {
        match self.context_stack.last().cloned() {
            Some(ContextKind::Task { full_path }) => self
                .tasks
                .get_mut(&full_path)
                .ok_or_else(|| context_error("Internal error: registered task not found")),
            Some(ContextKind::Group { .. }) | Some(ContextKind::Root) => Err(context_error(
                "actions() can only be used inside task(). Call task() first.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    fn build_child_path(&self, name: &str) -> Result<String, Box<EvalAltResult>> {
        match self.context_stack.last() {
            Some(ContextKind::Root) => Ok(name.to_string()),
            Some(ContextKind::Group { full_path }) => Ok(format!("{}.{}", full_path, name)),
            Some(ContextKind::Task { .. }) => {
                Err(context_error("Nested task() calls are not supported."))
            }
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    fn push_entry_to_parent(&mut self, entry: RegistryEntry) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.last().cloned() {
            Some(ContextKind::Root) => {
                self.root_entries.push(entry);
                Ok(())
            }
            Some(ContextKind::Group { full_path }) => {
                let group = self
                    .groups
                    .get_mut(&full_path)
                    .ok_or_else(|| context_error("Internal error: registered group not found"))?;
                group.entries.push(entry);
                Ok(())
            }
            Some(ContextKind::Task { .. }) => {
                Err(context_error("Nested task() calls are not supported."))
            }
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }
}

pub(crate) fn leaf_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

pub(crate) fn context_error(msg: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(msg.into().into(), Position::NONE).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Dynamic;
    fn dummy_fn_ptr() -> FnPtr {
        // The `FnPtr` is never invoked in these tests, so any dummy name is fine.
        // `FnPtr::new_unchecked` is unsafe and would require a valid context.
        // Using `FnPtr::new` keeps the tests safe while producing a callable pointer.
        FnPtr::new("dummy").unwrap()
    }

    #[test]
    fn task_registration_and_context() {
        let mut registry = TaskRegistry::new();

        registry.begin_task("build").unwrap();
        assert!(registry.tasks.contains_key("build"));

        registry.set_description("desc").unwrap();
        registry.set_actions(dummy_fn_ptr()).unwrap();

        registry.end_task().unwrap();

        let task = registry.tasks.get("build").unwrap();
        assert_eq!(task.description.as_deref(), Some("desc"));
        assert!(task.actions.is_some());
    }

    #[test]
    fn group_and_nested_task() {
        let mut registry = TaskRegistry::new();
        registry.begin_group("build_suite").unwrap();
        registry.set_description("suite").unwrap();

        registry.begin_task("build").unwrap();
        registry.end_task().unwrap();

        registry.begin_group("release_flow").unwrap();
        registry.begin_task("deploy").unwrap();
        registry.end_task().unwrap();
        registry.end_group().unwrap();

        registry.end_group().unwrap();

        assert!(registry.groups.contains_key("build_suite"));
        assert!(registry.groups.contains_key("build_suite.release_flow"));
        assert!(registry.tasks.contains_key("build_suite.build"));
        assert!(
            registry
                .tasks
                .contains_key("build_suite.release_flow.deploy")
        );
    }

    #[test]
    fn reject_task_redefinition() {
        let mut registry = TaskRegistry::new();
        registry.begin_task("build").unwrap();
        registry.end_task().unwrap();

        let err = registry.begin_task("build").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("is already defined"));
    }

    #[test]
    fn reject_group_redefinition_as_task() {
        let mut registry = TaskRegistry::new();
        registry.begin_group("ops").unwrap();
        registry.end_group().unwrap();

        let err = registry.begin_task("ops").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("is already defined as a group"));
    }

    #[test]
    fn nested_task_rejected() {
        let mut registry = TaskRegistry::new();
        registry.begin_task("outer").unwrap();
        let err = registry.begin_task("inner").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("Nested task() calls are not supported"));
    }

    #[test]
    fn description_outside_context_fails() {
        let mut registry = TaskRegistry::new();
        let err = registry.set_description("no context").unwrap_err();
        assert!(
            err.to_string()
                .contains("description() can only be used inside task() or group().")
        );
    }

    #[test]
    fn args_outside_task_fails() {
        let mut registry = TaskRegistry::new();
        let mut params = Map::new();
        params.insert("profile".into(), Dynamic::from("release"));

        let err = registry.set_args(params).unwrap_err();
        assert!(
            err.to_string()
                .contains("args() can only be used inside task().")
        );
    }
}
