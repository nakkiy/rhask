use super::model::{leaf_name, RegistryEntry, TaskRegistry};
use crate::logger::trace;

#[derive(Debug, Default, Clone)]
pub struct ListOutput {
    pub items: Vec<ListItem>,
    pub messages: Vec<ListMessage>,
}

impl ListOutput {
    fn push_message(&mut self, level: ListMessageLevel, text: impl Into<String>) {
        self.messages.push(ListMessage {
            level,
            text: text.into(),
        });
    }

    fn push_item(&mut self, item: ListItem) {
        self.items.push(item);
    }
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub kind: ListItemKind,
    pub depth: usize,
    pub name: String,
    pub description: Option<String>,
}

impl ListItem {
    fn group(depth: usize, name: String, description: Option<String>) -> Self {
        Self {
            kind: ListItemKind::Group,
            depth,
            name,
            description,
        }
    }

    fn task(depth: usize, name: String, description: Option<String>) -> Self {
        Self {
            kind: ListItemKind::Task,
            depth,
            name,
            description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListItemKind {
    Group,
    Task,
}

#[derive(Debug, Clone)]
pub struct ListMessage {
    pub level: ListMessageLevel,
    pub text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMessageLevel {
    Info,
    Warn,
    Error,
}

impl TaskRegistry {
    pub fn list(&self, group: Option<&str>) {
        let output = self.collect_list_output(group);
        crate::printer::print_list(&output);
    }

    fn collect_list_output(&self, group: Option<&str>) -> ListOutput {
        let mut output = ListOutput::default();

        if let Some(path) = group {
            trace!("list request for group '{}'", path);
            match self.resolve_group(path) {
                GroupLookup::Found(full_path) => {
                    trace!("resolved group '{}' -> '{}'", path, full_path);
                    self.collect_group(&full_path, 0, &mut output);
                }
                GroupLookup::Ambiguous(paths) => {
                    output.push_message(
                        ListMessageLevel::Warn,
                        format!("Group '{}' matches multiple candidates:", path),
                    );
                    for candidate in paths {
                        output.push_message(ListMessageLevel::Warn, format!("  - {}", candidate));
                    }
                    output.push_message(
                        ListMessageLevel::Warn,
                        "Please use the fully-qualified name (e.g. parent.child).",
                    );
                }
                GroupLookup::NotFound => {
                    output.push_message(
                        ListMessageLevel::Warn,
                        format!("Group '{}' does not exist.", path),
                    );
                }
            }
            return output;
        }

        if self.root_entries.is_empty() {
            for (full_path, task) in self.tasks.iter() {
                output.push_item(ListItem::task(
                    0,
                    leaf_name(full_path).to_string(),
                    task.description.clone(),
                ));
            }
            return output;
        }

        for entry in &self.root_entries {
            self.collect_entry(entry, 0, &mut output);
        }

        output
    }

    fn collect_entry(&self, entry: &RegistryEntry, depth: usize, output: &mut ListOutput) {
        match entry {
            RegistryEntry::Task(full_path) => self.collect_task(full_path, depth, output),
            RegistryEntry::Group(full_path) => self.collect_group(full_path, depth, output),
        }
    }

    fn collect_task(&self, full_path: &str, depth: usize, output: &mut ListOutput) {
        if let Some(task) = self.tasks.get(full_path) {
            output.push_item(ListItem::task(
                depth,
                leaf_name(full_path).to_string(),
                task.description.clone(),
            ));
        } else {
            output.push_item(ListItem::task(
                depth,
                leaf_name(full_path).to_string(),
                None,
            ));
        }
    }

    fn collect_group(&self, full_path: &str, depth: usize, output: &mut ListOutput) {
        if let Some(group) = self.groups.get(full_path) {
            let name = leaf_name(full_path).to_string();
            output.push_item(ListItem::group(depth, name, group.description.clone()));
            for entry in &group.entries {
                self.collect_entry(entry, depth + 1, output);
            }
        } else {
            output.push_item(ListItem::group(
                depth,
                leaf_name(full_path).to_string(),
                None,
            ));
        }
    }
}

enum GroupLookup {
    Found(String),
    Ambiguous(Vec<String>),
    NotFound,
}

impl TaskRegistry {
    fn resolve_group(&self, identifier: &str) -> GroupLookup {
        let trimmed = identifier.trim();
        if trimmed.is_empty() {
            return GroupLookup::NotFound;
        }
        trace!("resolving group '{}'", trimmed);

        if self.groups.contains_key(trimmed) {
            trace!("group '{}' found exact match", trimmed);
            return GroupLookup::Found(trimmed.to_string());
        }

        if trimmed.contains('.') {
            trace!("group '{}' treated as dotted path but not found", trimmed);
            return GroupLookup::NotFound;
        }

        let matches: Vec<String> = self
            .groups
            .keys()
            .filter(|full_path| leaf_name(full_path) == trimmed)
            .cloned()
            .collect();

        match matches.len() {
            0 => GroupLookup::NotFound,
            1 => {
                let full_path = matches.into_iter().next().unwrap();
                trace!(
                    "group '{}' resolved to unique match '{}'",
                    trimmed,
                    full_path
                );
                GroupLookup::Found(full_path)
            }
            _ => {
                trace!(
                    "group '{}' resolved to ambiguous matches {:?}",
                    trimmed,
                    matches
                );
                GroupLookup::Ambiguous(matches)
            }
        }
    }
}
