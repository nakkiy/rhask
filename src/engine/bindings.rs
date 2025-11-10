use rhai::{Dynamic, Engine};
use std::sync::{Arc, Mutex};

use super::api;
use super::core::ExecutionState;
use super::runtime::{RegistryRef, RuntimeHandle};

pub fn register_all(
    engine: &mut Engine,
    registry: RegistryRef,
    exec_state: Arc<Mutex<ExecutionState>>,
) {
    let runtime = RuntimeHandle::new(registry, exec_state);
    engine.set_default_tag(Dynamic::from(runtime));
    api::register(engine);
}
