use rhai::{EvalAltResult, FnPtr, ImmutableString, Map};
use std::fs;
use std::path::{Path, PathBuf};

use crate::task::builder::{GroupBuilder, TaskBuilder};
use crate::task::model::{context_error, ParameterSpec, RegistryEntry};
use crate::task::registry::TaskRegistry;

#[derive(Clone, Debug)]
pub(crate) enum ContextFrame {
    Root,
    Group(GroupBuilder),
    Task(TaskBuilder),
}

#[derive(Clone)]
pub struct BuildStack {
    context_stack: Vec<ContextFrame>,
    script_root: Option<PathBuf>,
}

impl Default for BuildStack {
    fn default() -> Self {
        Self {
            context_stack: vec![ContextFrame::Root],
            script_root: None,
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
        self.script_root = None;
    }

    pub fn set_script_root(&mut self, root: PathBuf) {
        self.script_root = Some(root);
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
            Some(ContextFrame::Task(builder)) => {
                let (full_path, task) = builder.build();
                registry.insert_task_entry(full_path.clone(), task);
                self.attach_entry_to_parent(registry, RegistryEntry::Task(full_path))
            }
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
            Some(ContextFrame::Group(builder)) => {
                let (full_path, group) = builder.build();
                registry.insert_group_entry(full_path.clone(), group);
                self.attach_entry_to_parent(registry, RegistryEntry::Group(full_path))
            }
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
        match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => {
                builder.set_actions(func);
                Ok(())
            }
            Some(ContextFrame::Group(_)) | Some(ContextFrame::Root) => Err(context_error(
                "actions() can only be used inside task(). Call task() first.",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub fn set_args(&mut self, params: Map) -> Result<(), Box<EvalAltResult>> {
        let builder = match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => builder,
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

        let specs = entries
            .into_iter()
            .map(|(name, default)| ParameterSpec { name, default })
            .collect();
        builder.set_params(specs);
        Ok(())
    }

    pub fn set_description(&mut self, desc: &str) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => {
                builder.set_description(desc);
                Ok(())
            }
            Some(ContextFrame::Group(builder)) => {
                builder.set_description(desc);
                Ok(())
            }
            Some(ContextFrame::Root) => Err(context_error(
                "description() can only be used inside task() or group().",
            )),
            None => Err(context_error("context mismatch: context stack is empty.")),
        }
    }

    pub fn set_directory(&mut self, path: &str) -> Result<(), Box<EvalAltResult>> {
        match self.context_stack.last() {
            Some(ContextFrame::Task(builder)) => {
                if builder.has_working_dir() {
                    return Err(context_error("dir() can only be defined once per task()."));
                }
            }
            Some(ContextFrame::Group(_)) | Some(ContextFrame::Root) => {
                return Err(context_error("dir() can only be used inside task()."));
            }
            None => {
                return Err(context_error("context mismatch: context stack is empty."));
            }
        }

        let resolved = self.resolve_directory(path)?;

        match self.context_stack.last_mut() {
            Some(ContextFrame::Task(builder)) => {
                builder.set_working_dir(resolved);
            }
            _ => {
                return Err(context_error(
                    "dir() context mismatch while applying working directory.",
                ));
            }
        }
        Ok(())
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
                registry.push_root_entry(entry);
                Ok(())
            }
            Some(ContextFrame::Group(builder)) => {
                builder.add_entry(entry);
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
        if registry.contains_task(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Task(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Task '{}' is already defined.",
                full_path
            )))
        } else if registry.contains_group(full_path)
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
        if registry.contains_group(full_path)
            || self
                .context_stack
                .iter()
                .any(|frame| matches!(frame, ContextFrame::Group(builder) if builder.full_path == full_path))
        {
            Err(context_error(format!(
                "Group '{}' is already defined.",
                full_path
            )))
        } else if registry.contains_task(full_path)
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

    fn resolve_directory(&self, path: &str) -> Result<PathBuf, Box<EvalAltResult>> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(context_error("dir() requires a non-empty path."));
        }
        let raw = Path::new(trimmed);
        let candidate = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            let root = self.script_root.clone().ok_or_else(|| {
                context_error("dir() cannot be used before the rhaskfile root is known.")
            })?;
            root.join(raw)
        };

        let normalized = candidate.canonicalize().map_err(|err| {
            context_error(format!(
                "dir(): unable to resolve '{}': {}",
                candidate.display(),
                err
            ))
        })?;

        let metadata = fs::metadata(&normalized).map_err(|err| {
            context_error(format!(
                "dir(): unable to inspect '{}': {}",
                normalized.display(),
                err
            ))
        })?;

        if !metadata.is_dir() {
            return Err(context_error(format!(
                "dir(): '{}' is not a directory.",
                normalized.display()
            )));
        }

        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dir_sets_absolute_path_from_relative_input() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("workspace");
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).expect("create scripts dir");

        let mut stack = BuildStack::new();
        stack.set_script_root(root.clone());
        let mut registry = TaskRegistry::new();
        stack.begin_task(&registry, "demo").expect("begin task");
        stack.set_directory("scripts").expect("set dir");
        stack
            .end_task(&mut registry)
            .expect("end task with dir configured");

        let task = registry.task("demo").expect("task stored");
        let expected = scripts
            .canonicalize()
            .expect("canonicalize scripts directory");
        assert_eq!(task.working_dir.as_ref(), Some(&expected));
    }

    #[test]
    fn dir_rejects_second_invocation_in_same_task() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("scripts")).expect("create scripts dir");

        let mut stack = BuildStack::new();
        stack.set_script_root(root);
        let registry = TaskRegistry::new();
        stack.begin_task(&registry, "dup").expect("begin task");
        stack.set_directory("scripts").expect("first dir()");
        assert!(stack.set_directory("scripts").is_err());
    }

    #[test]
    fn dir_requires_directory_to_exist() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).expect("create workspace root");

        let mut stack = BuildStack::new();
        stack.set_script_root(root);
        let registry = TaskRegistry::new();
        stack.begin_task(&registry, "missing").expect("begin task");
        assert!(stack.set_directory("unknown_dir").is_err());
    }

    #[test]
    fn dir_requires_task_scope() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().to_path_buf();
        let mut stack = BuildStack::new();
        stack.set_script_root(root);
        assert!(stack.set_directory("scripts").is_err());
    }

    #[test]
    fn dir_accepts_absolute_path() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("workspace");
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).expect("create scripts dir");

        let mut stack = BuildStack::new();
        stack.set_script_root(root);
        let mut registry = TaskRegistry::new();
        stack.begin_task(&registry, "abs").expect("begin task");
        let abs = scripts.canonicalize().expect("canonicalize scripts dir");
        let raw = abs.to_str().expect("utf8 path").to_string();
        stack.set_directory(&raw).expect("set dir using absolute");
        stack.end_task(&mut registry).expect("end task");
        let task = registry.task("abs").expect("task stored");
        assert_eq!(task.working_dir.as_ref(), Some(&abs));
    }
}
