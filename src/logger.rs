// src/logger.rs
use env_logger::{Builder, Env, Target};
use std::io::Write;

pub fn init() {
    let env = if cfg!(debug_assertions) {
        Env::default().default_filter_or("debug")
    } else {
        Env::default().default_filter_or("off")
    };

    let mut builder = Builder::from_env(env);

    builder.target(Target::Stderr).format(|buf, record| {
        writeln!(
            buf,
            "[{:>5} {}:{}] {}",
            record.level(),
            record.file().unwrap_or("unknown"),
            record.line().unwrap_or(0),
            record.args()
        )
    });

    if builder.try_init().is_err() {
        log::debug!("logger already initialized");
    }
}

/// Re-export logging macros
#[allow(unused_imports)]
pub use log::{debug, error, info, trace, warn};
