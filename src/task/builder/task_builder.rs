use rhai::FnPtr;

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

    pub fn set_actions(&mut self, func: FnPtr) {
        self.task.actions = Some(func);
    }

    pub fn set_params(&mut self, params: Vec<ParameterSpec>) {
        self.task.params = params;
    }

    pub fn build(self) -> (String, Task) {
        (self.full_path, self.task)
    }
}
