//! MCP command queue — allows MCP tools to send actions to the main thread.

use std::collections::VecDeque;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use txv_core::run::Waker;

/// A request from an MCP tool to mutate app state.
pub struct McpRequest {
    pub action: McpAction,
    pub reply: std::sync::mpsc::SyncSender<Result<Value, String>>,
}

/// Actions the MCP server can request.
pub enum McpAction {
    /// Execute a Tcl command string (same as typing in editor + F9).
    RunCommand { script: String },
    /// Set editor content.
    SetEditorContent { content: String },
    /// Get editor content (read via main thread for consistency).
    GetEditorContent,
    /// List lineage nodes.
    ListNodes,
    /// Preview table data.
    PreviewTable { name: String, limit: usize },
}

/// Shared command queue + waker.
#[derive(Clone)]
pub struct McpCommandQueue {
    queue: Arc<Mutex<VecDeque<McpRequest>>>,
    waker: Waker,
}

impl McpCommandQueue {
    pub fn new(waker: Waker) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            waker,
        }
    }

    /// Push a request, wake event loop, wait for reply.
    pub fn send(&self, action: McpAction) -> Result<Value, String> {
        let (tx, rx) = sync_channel(1);
        let req = McpRequest { action, reply: tx };
        {
            let mut q = self.queue.lock().map_err(|_| "queue poisoned")?;
            q.push_back(req);
        }
        self.waker.wake();
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| format!("MCP command timeout: {e}"))?
    }

    /// Drain pending requests (called from main thread).
    pub fn drain(&self) -> Vec<McpRequest> {
        if let Ok(mut q) = self.queue.lock() {
            q.drain(..).collect()
        } else {
            Vec::new()
        }
    }
}
