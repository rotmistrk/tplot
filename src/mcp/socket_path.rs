//! Socket path computation for the MCP server.

use std::env;
use std::path::{Path, PathBuf};

/// Compute the Unix socket path: `$XDG_RUNTIME_DIR/tplot-{hash}.sock`
pub fn socket_path(root: &Path) -> PathBuf {
    let dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
    let hash = simple_hash(root.to_string_lossy().as_bytes());
    PathBuf::from(dir).join(format!("tplot-{hash}.sock"))
}

fn simple_hash(data: &[u8]) -> String {
    let mut h: u64 = 5381;
    for &b in data {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{h:016x}")
}
