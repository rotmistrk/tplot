//! Application state.

use std::path::PathBuf;

use crate::engine::Engine;
use crate::registry::Registry;
use crate::scripting::ScriptEngine;

pub struct AppState {
    pub(crate) root_dir: PathBuf,
    engine: Engine,
    scripting: ScriptEngine,
    pub registry: Registry,
    /// Node name pending deletion (awaiting confirmation).
    pub(crate) pending_delete: Option<String>,
    /// Background job manager.
    pub(crate) jobs: crate::jobs::JobManager,
}

impl AppState {
    pub fn new(root_dir: PathBuf) -> Self {
        let engine = Engine::open(&root_dir).unwrap_or_else(|e| {
            log::error!("DuckDB open failed at {}: {e} — using in-memory", root_dir.display());
            Engine::open_memory().expect("in-memory DB")
        });
        let scripting = ScriptEngine::new();
        let registry = Registry::open(&root_dir);
        log::info!("tplot: root={}, nodes={}", root_dir.display(), registry.nodes().len());
        Self {
            root_dir,
            engine,
            scripting,
            registry,
            pending_delete: None,
            jobs: crate::jobs::JobManager::new(),
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
