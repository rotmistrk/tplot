//! Application state.

use std::path::PathBuf;

use crate::engine::Engine;
use crate::scripting::ScriptEngine;

pub(crate) struct AppState {
    #[allow(dead_code)]
    root_dir: PathBuf,
    #[allow(dead_code)]
    engine: Engine,
    #[allow(dead_code)]
    scripting: ScriptEngine,
}

impl AppState {
    pub(crate) fn new(root_dir: PathBuf) -> Self {
        let engine = Engine::open(&root_dir).unwrap_or_else(|e| {
            log::error!("Failed to open DuckDB: {e}, using in-memory");
            Engine::open_memory().expect("in-memory DB")
        });
        let scripting = ScriptEngine::new();
        Self {
            root_dir,
            engine,
            scripting,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    #[allow(dead_code)]
    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    #[allow(dead_code)]
    pub(crate) fn scripting(&mut self) -> &mut ScriptEngine {
        &mut self.scripting
    }
}
