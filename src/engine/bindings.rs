use rhai::{packages::Package, Dynamic, Engine};
use std::sync::{Arc, Mutex};

use super::api;
use super::core::ExecutionState;
use super::runtime::{BuildStackRef, RegistryRef, RuntimeHandle};
use rhai_process::{Config, ProcessPackage};

pub fn register_all(
    engine: &mut Engine,
    registry: RegistryRef,
    exec_state: Arc<Mutex<ExecutionState>>,
    build_stack: BuildStackRef,
) {
    let runtime = RuntimeHandle::new(registry, exec_state, build_stack);
    engine.set_default_tag(Dynamic::from(runtime));
    ProcessPackage::new(Config::default()).register_into_engine(engine);
    api::register(engine);
}
