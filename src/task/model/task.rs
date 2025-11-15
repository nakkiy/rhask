use rhai::FnPtr;
use std::path::PathBuf;

#[derive(Clone, Default, Debug)]
pub struct Task {
    pub description: Option<String>,
    pub actions: Option<FnPtr>,
    pub params: Vec<ParameterSpec>,
    pub working_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct ParameterSpec {
    pub name: String,
    pub default: Option<String>,
}
