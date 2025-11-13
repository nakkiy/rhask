#[derive(Clone, Default, Debug)]
pub struct Group {
    pub description: Option<String>,
    pub entries: Vec<RegistryEntry>,
}

#[derive(Clone, Debug)]
pub enum RegistryEntry {
    Task(String),
    Group(String),
}
