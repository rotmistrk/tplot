//! Test harness for tplot — same pattern as kairn tests.

#![allow(dead_code)]

use std::path::Path;

use tempfile::TempDir;
use txv_core::event::{KeyCode, KeyMod};
use txv_core::program::Program;
use txv_core::run::MockBackend;
use txv_widgets::sidekick_manager::SidekickManager;

use tplot::app::AppState;
use tplot::handler::handle_command;
use tplot::status::build_status_bar;
use tplot::workspace::build_workspace;

pub struct TestHarness {
    pub program: Program,
    pub backend: MockBackend,
    pub state: AppState,
}

impl TestHarness {
    pub fn new(root_dir: &Path) -> Self {
        Self::with_size(root_dir, 80, 24)
    }

    pub fn with_size(root_dir: &Path, width: u16, height: u16) -> Self {
        let ws = build_workspace(root_dir);
        let status = build_status_bar(&ws);
        let mut program = Program::new(Box::new(status), Box::new(ws));
        program.insert_named("sidekick", Box::new(SidekickManager::new()));
        let state = AppState::new(root_dir.to_path_buf());
        tplot::handler::initial_refresh(program.desktop_mut(), &state.registry);
        let backend = MockBackend::new(width, height);
        Self {
            program,
            backend,
            state,
        }
    }

    pub fn inject_key(&mut self, code: KeyCode, mods: KeyMod) {
        self.backend.inject_key(code, mods);
    }

    pub fn inject_str(&mut self, s: &str) {
        self.backend.inject_str(s);
    }

    pub fn run_cycles(&mut self, n: usize) {
        let state = &mut self.state;
        self.program.run_cycles(
            &mut self.backend,
            &mut |ctx| {
                handle_command(ctx, state);
            },
            n,
        );
    }

    pub fn screen_text(&self) -> String {
        self.backend.screen_text()
    }

    pub fn contains(&self, text: &str) -> bool {
        self.backend.contains(text)
    }

    pub fn content_contains(&self, text: &str) -> bool {
        self.backend.content_contains(text)
    }

    pub fn row(&self, y: u16) -> String {
        self.backend.row(y)
    }
}

pub fn temp_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }
    dir
}
