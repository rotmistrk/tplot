//! Node status and metadata — common to all node types.

use std::time::{Duration, SystemTime};

/// Current execution state of a node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NodeStatus {
    /// Never been run.
    Empty,
    /// Result matches current inputs.
    UpToDate,
    /// Upstream changed, needs re-run.
    Dirty,
    /// Currently executing.
    Running,
    /// Last execution failed.
    Error(String),
}

/// Observable and estimated metadata for a node.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeMeta {
    // Observed (after run)
    pub(crate) data_bytes: Option<u64>,
    pub(crate) row_count: Option<u64>,
    pub(crate) last_run_duration: Option<Duration>,
    pub(crate) last_run_at: Option<SystemTime>,

    // Estimated (from history or upstream)
    pub(crate) estimated_bytes: Option<u64>,
    pub(crate) estimated_run_cost: Option<Duration>,
}

impl NodeStatus {
    pub(crate) fn icon(&self) -> &str {
        match self {
            Self::Empty => "○",
            Self::UpToDate => "✓",
            Self::Dirty => "⚠",
            Self::Running => ">",
            Self::Error(_) => "✗",
        }
    }
}
