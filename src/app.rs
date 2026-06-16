//! Application state.

use std::path::PathBuf;

use crate::engine::Engine;
use crate::registry::Registry;
use crate::scripting::ScriptEngine;

pub(crate) struct AppState {
    #[allow(dead_code)]
    root_dir: PathBuf,
    engine: Engine,
    scripting: ScriptEngine,
    pub(crate) registry: Registry,
}

impl AppState {
    pub(crate) fn new(root_dir: PathBuf) -> Self {
        let engine = Engine::open(&root_dir).unwrap_or_else(|e| {
            log::error!("Failed to open DuckDB: {e}, using in-memory");
            Engine::open_memory().expect("in-memory DB")
        });
        let scripting = ScriptEngine::new();
        let registry = Registry::open(&root_dir);
        Self {
            root_dir,
            engine,
            scripting,
            registry,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    pub(crate) fn scripting(&mut self) -> &mut ScriptEngine {
        &mut self.scripting
    }
}
