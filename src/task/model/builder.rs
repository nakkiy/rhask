use rhai::{EvalAltResult, FnPtr, ImmutableString, Map};

use super::registry::TaskRegistry;
use super::types::{context_error, Group, ParameterSpec, RegistryEntry, Task};

#[derive(Clone, Debug)]
pub(crate) enum ContextFrame {
    Root,
    Group(GroupBuilder),
    Task(TaskBuilder),
}

#[derive(Clone, Debug)]
pub(crate) struct TaskBuilder {
    pub(super) full_path: String,
    pub(super) task: Task,
}

impl TaskBuilder {
    fn new(full_path: String) -> Self {
        Self {
            full_path,
            task: Task::default(),
        }
    }

    fn task_mut(&mut self) -> &mut Task {
        &mut self.task
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GroupBuilder {
    pub(super) full_path: String,
    pub(super) group: Group,
}

impl GroupBuilder {
    fn new(full_path: String) -> Self {
        Self {
            full_path,
            group: Group::default(),
        }
    }
}

#[derive(Clone)]
pub struct BuildStack {
    context_stack: Vec<ContextFrame>,
}

impl Default for BuildStack {
    fn default() -> Self {
        Self {
            context_stack: vec![ContextFrame::Root],
        }
    }
}

impl BuildStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.context_stack.clear();
        self.context_stack.push(ContextFrame::Root);
    }

    pub(crate) fn begin_task(
        &mut self,
        registry: &TaskRegistry,
        name: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(context_error("Task name cannot be empty."));
        }

        let full_path = self.build_child_path(name)?;

        self.ensure_task_name_available(registry, &full_path)?;

        self.context_stack
            .push(ContextFrame::Task(TaskBuilder::new(full_path)));
        Ok(())
    }

    pub(crate) fn end_task(
        &mut self,
        registry: &mut TaskRegistry,
    ) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.pop() {
            Some(ContextFrame::Task(builder)) => self.finalize_task(registry, builder),
            Some(ContextFrame::Group(_)) => Err(context_error(
                "context mismatch: end_task() called while inside group().",
            )),
            Some(ContextFrame::Root) => Err(context_error(
                "context mismatch: end_task() called before task() was started.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub(crate) fn begin_group(
        &mut self,
        registry: &TaskRegistry,
        name: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(context_error("Group name cannot be empty."));
        }

        let full_path = self.build_child_path(name)?;

        self.ensure_group_name_available(registry, &full_path)?;

        self.context_stack
            .push(ContextFrame::Group(GroupBuilder::new(full_path)));
        Ok(())
    }

    pub(crate) fn end_group(
        &mut self,
        registry: &mut TaskRegistry,
    ) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.pop() {
            Some(ContextFrame::Group(builder)) => self.finalize_group(registry, builder),
            Some(ContextFrame::Task(_)) => Err(context_error(
                "context mismatch: task() scope was not closed before ending group().",
            )),
            Some(ContextFrame::Root) => Err(context_error(
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
        let task = match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => builder.task_mut(),
            Some(ContextFrame::Group(_)) | Some(ContextFrame::Root) => {
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
        match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => {
                builder.task.description = Some(desc.to_string());
                Ok(())
            }
            Some(ContextFrame::Group(builder)) => {
                builder.group.description = Some(desc.to_string());
                Ok(())
            }
            Some(ContextFrame::Root) => Err(context_error(
                "description() can only be used inside task() or group().",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub(crate) fn current_task_mut(&mut self) -> Result<&mut Task, Box<EvalAltResult>> {
        match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => Ok(builder.task_mut()),
            Some(ContextFrame::Group(_)) | Some(ContextFrame::Root) => Err(context_error(
                "actions() can only be used inside task(). Call task() first.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    fn build_child_path(&self, name: &str) -> Result<String, Box<EvalAltResult>> {
        match self.context_stack.last() {
            Some(ContextFrame::Root) => Ok(name.to_string()),
            Some(ContextFrame::Group(builder)) => Ok(format!("{}.{}", builder.full_path, name)),
            Some(ContextFrame::Task(_)) => {
                Err(context_error("Nested task() calls are not supported."))
            }
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    fn attach_entry_to_parent(
        &mut self,
        registry: &mut TaskRegistry,
        entry: RegistryEntry,
    ) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.last_mut() {
            Some(ContextFrame::Root) => {
                registry.root_entries.push(entry);
                Ok(())
            }
            Some(ContextFrame::Group(builder)) => {
                builder.group.entries.push(entry);
                Ok(())
            }
            Some(ContextFrame::Task(_)) => {
                Err(context_error("Nested task() calls are not supported."))
            }
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    fn ensure_task_name_available(
        &self,
        registry: &TaskRegistry,
        full_path: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        if registry.tasks.contains_key(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Task(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Task '{}' is already defined.",
                full_path
            )))
        } else if registry.groups.contains_key(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Group(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Task '{}' is already defined as a group.",
                full_path
            )))
        } else {
            Ok(())
        }
    }

    fn ensure_group_name_available(
        &self,
        registry: &TaskRegistry,
        full_path: &str,
    ) -> Result<(), Box<EvalAltResult>> {
        if registry.groups.contains_key(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Group(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Group '{}' is already defined.",
                full_path
            )))
        } else if registry.tasks.contains_key(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Task(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Group '{}' is already defined as a task.",
                full_path
            )))
        } else {
            Ok(())
        }
    }

    fn finalize_task(
        &mut self,
        registry: &mut TaskRegistry,
        builder: TaskBuilder,
    ) -> Result<(), Box<EvalAltResult>> {
        let TaskBuilder { full_path, task } = builder;
        registry.tasks.insert(full_path.clone(), task);
        self.attach_entry_to_parent(registry, RegistryEntry::Task(full_path))
    }

    fn finalize_group(
        &mut self,
        registry: &mut TaskRegistry,
        builder: GroupBuilder,
    ) -> Result<(), Box<EvalAltResult>> {
        let GroupBuilder { full_path, group } = builder;
        registry.groups.insert(full_path.clone(), group);
        self.attach_entry_to_parent(registry, RegistryEntry::Group(full_path))
    }
}
