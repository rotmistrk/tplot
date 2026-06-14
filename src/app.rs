//! Application state.

use std::path::PathBuf;

#[allow(dead_code)]
pub(crate) struct AppState {
    root_dir: PathBuf,
}

impl AppState {
    pub(crate) fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}
