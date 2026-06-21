//! Session persistence — save/restore cmd editor content across restarts.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const STATE_FILE: &str = ".tplot.state";

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct SessionState {
    /// Cmd editor buffer content.
    #[serde(default)]
    pub(crate) editor_content: String,
}

/// Save session state.
pub(crate) fn save_session(root_dir: &Path, state: &SessionState) {
    let path = root_dir.join(STATE_FILE);
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(path, json);
    }
}

/// Load session state. Returns None if missing/corrupt.
pub(crate) fn load_session(root_dir: &Path) -> Option<SessionState> {
    let path = root_dir.join(STATE_FILE);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}
