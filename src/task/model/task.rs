use rhai::FnPtr;

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
