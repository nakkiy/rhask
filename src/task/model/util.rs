use rhai::{EvalAltResult, Position};

pub(crate) fn leaf_name(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

pub(crate) fn context_error(msg: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(msg.into().into(), Position::NONE).into()
}
