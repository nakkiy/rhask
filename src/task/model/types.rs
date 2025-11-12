use rhai::{EvalAltResult, FnPtr, Position};

#[derive(Clone, Default, Debug)]
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

pub(crate) fn leaf_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

pub(crate) fn context_error(msg: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(msg.into().into(), Position::NONE).into()
}
