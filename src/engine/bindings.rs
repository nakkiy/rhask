use rhai::{Dynamic, Engine};
use std::sync::{Arc, Mutex};

use super::api;
use super::core::ExecutionState;
use super::runtime::{BuildStackRef, RegistryRef, RuntimeHandle};

pub fn register_all(
    engine: &mut Engine,
    registry: RegistryRef,
    exec_state: Arc<Mutex<ExecutionState>>,
    build_stack: BuildStackRef,
) {
    let runtime = RuntimeHandle::new(registry, exec_state, build_stack);
    engine.set_default_tag(Dynamic::from(runtime));
    api::register(engine);
}
