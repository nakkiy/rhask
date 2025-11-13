use crate::task::model::{Group, RegistryEntry};

#[derive(Clone, Debug)]
pub struct GroupBuilder {
    pub(crate) full_path: String,
    group: Group,
}

impl GroupBuilder {
    pub fn new(full_path: String) -> Self {
        Self {
            full_path,
            group: Group::default(),
        }
    }

    pub fn set_description(&mut self, desc: &str) {
        self.group.description = Some(desc.to_string());
    }

    pub fn add_entry(&mut self, entry: RegistryEntry) {
        self.group.entries.push(entry);
    }

    pub fn build(self) -> (String, Group) {
        (self.full_path, self.group)
    }
}
