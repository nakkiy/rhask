use rhai::FnPtr;
use std::path::PathBuf;

use crate::task::model::{ParameterSpec, Task};

#[derive(Clone, Debug)]
pub struct TaskBuilder {
    pub(crate) full_path: String,
    task: Task,
}

impl TaskBuilder {
    pub fn new(full_path: String) -> Self {
        Self {
            full_path,
            task: Task::default(),
        }
    }

    pub fn set_description(&mut self, desc: &str) {
        self.task.description = Some(desc.to_string());
    }

    pub fn has_description(&self) -> bool {
        self.task.description.is_some()
    }

    pub fn set_actions(&mut self, func: FnPtr) {
        self.task.actions = Some(func);
    }

    pub fn has_actions(&self) -> bool {
        self.task.actions.is_some()
    }

    pub fn set_params(&mut self, params: Vec<ParameterSpec>) {
        self.task.params = params;
    }

    pub fn has_params(&self) -> bool {
        !self.task.params.is_empty()
    }

    pub fn set_working_dir(&mut self, dir: PathBuf) {
        self.task.working_dir = Some(dir);
    }

    pub fn has_working_dir(&self) -> bool {
        self.task.working_dir.is_some()
    }

    pub fn build(self) -> (String, Task) {
        (self.full_path, self.task)
    }
}
