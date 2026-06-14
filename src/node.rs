//! Node model — represents a single node in the lineage DAG.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// State of a node's materialized data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum NodeState {
    /// User is working here, commands logging to script.
    Active,
    /// Data exists and matches current script.
    Materialized,
    /// Sealed. Edits auto-branch.
    Frozen,
    /// Script inherited from variant, not yet run.
    Ghost,
    /// Script edited, data doesn't match.
    Stale,
    /// Computation in progress.
    Running,
    /// Script execution failed.
    Error,
}

/// Metadata stored in meta.toml for each node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct NodeMeta {
    /// Display name.
    pub(crate) name: String,
    /// Parent node directory name (None for root nodes).
    pub(crate) parent: Option<String>,
    /// Additional dependencies (for joins — other node dir names).
    #[serde(default)]
    pub(crate) also_depends: Vec<String>,
    /// If this is a variant, references the original node dir name.
    pub(crate) variant_of: Option<String>,
    /// Current state.
    pub(crate) state: NodeState,
    /// When the node was created (ISO 8601).
    pub(crate) created: Option<String>,
    /// Last successful run timestamp.
    pub(crate) last_run: Option<String>,
    /// User comments.
    #[serde(default)]
    pub(crate) comments: String,
    /// Size tracking.
    #[serde(default)]
    pub(crate) size: SizeInfo,
    /// Timing tracking.
    #[serde(default)]
    pub(crate) timing: TimingInfo,
}

/// Disk size tracking for a node.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SizeInfo {
    /// Current node data size in bytes.
    pub(crate) data_bytes: u64,
    /// Cumulative descendants data size.
    pub(crate) descendants_bytes: u64,
    /// Historical peak size.
    pub(crate) peak_bytes: u64,
}

/// Execution timing for a node.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TimingInfo {
    /// Last run duration in seconds.
    pub(crate) last_run_secs: f64,
    /// Total cumulative run time.
    pub(crate) total_run_secs: f64,
    /// Number of times the script has been executed.
    pub(crate) run_count: u32,
}

#[allow(dead_code)]
impl NodeMeta {
    /// Create metadata for a new root node.
    pub(crate) fn new_root(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
            also_depends: vec![],
            variant_of: None,
            state: NodeState::Active,
            created: Some(now_iso()),
            last_run: None,
            comments: String::new(),
            size: SizeInfo::default(),
            timing: TimingInfo::default(),
        }
    }

    /// Create metadata for a child node.
    pub(crate) fn new_child(name: &str, parent_dir: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: Some(parent_dir.to_string()),
            also_depends: vec![],
            variant_of: None,
            state: NodeState::Ghost,
            created: Some(now_iso()),
            last_run: None,
            comments: String::new(),
            size: SizeInfo::default(),
            timing: TimingInfo::default(),
        }
    }

    /// Create metadata for a variant (branched edit).
    pub(crate) fn new_variant(name: &str, parent_dir: &str, original_dir: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: Some(parent_dir.to_string()),
            also_depends: vec![],
            variant_of: Some(original_dir.to_string()),
            state: NodeState::Ghost,
            created: Some(now_iso()),
            last_run: None,
            comments: String::new(),
            size: SizeInfo::default(),
            timing: TimingInfo::default(),
        }
    }
}

/// A node on disk with its path and metadata.
#[allow(dead_code)]
pub(crate) struct Node {
    /// Directory name (used as ID in references).
    pub(crate) dir_name: String,
    /// Full path to the node directory.
    pub(crate) path: PathBuf,
    /// Parsed metadata.
    pub(crate) meta: NodeMeta,
}

#[allow(dead_code)]
impl Node {
    /// Load a node from its directory.
    pub(crate) fn load(node_dir: &Path) -> Result<Self, String> {
        let dir_name = node_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "invalid node dir".to_string())?
            .to_string();
        let meta_path = node_dir.join("meta.toml");
        let content = std::fs::read_to_string(&meta_path).map_err(|e| format!("read meta.toml: {e}"))?;
        let meta: NodeMeta = toml::from_str(&content).map_err(|e| format!("parse meta.toml: {e}"))?;
        Ok(Self {
            dir_name,
            path: node_dir.to_path_buf(),
            meta,
        })
    }

    /// Create a new node on disk.
    pub(crate) fn create(nodes_dir: &Path, dir_name: &str, meta: NodeMeta) -> Result<Self, String> {
        let node_dir = nodes_dir.join(dir_name);
        std::fs::create_dir_all(&node_dir).map_err(|e| format!("create dir: {e}"))?;
        std::fs::create_dir_all(node_dir.join("views")).map_err(|e| format!("create views: {e}"))?;
        std::fs::create_dir_all(node_dir.join("data")).map_err(|e| format!("create data: {e}"))?;

        let meta_str = toml::to_string_pretty(&meta).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(node_dir.join("meta.toml"), meta_str).map_err(|e| format!("write meta: {e}"))?;

        Ok(Self {
            dir_name: dir_name.to_string(),
            path: node_dir,
            meta,
        })
    }

    /// Save updated metadata.
    pub(crate) fn save_meta(&self) -> Result<(), String> {
        let meta_str = toml::to_string_pretty(&self.meta).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(self.path.join("meta.toml"), meta_str).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }

    /// Read the node's script (if it exists).
    pub(crate) fn read_script(&self) -> Option<String> {
        let sql = self.path.join("script.sql");
        if sql.exists() {
            return std::fs::read_to_string(&sql).ok();
        }
        let tcl = self.path.join("script.tcl");
        if tcl.exists() {
            return std::fs::read_to_string(&tcl).ok();
        }
        None
    }

    /// Check if this node has materialized data.
    pub(crate) fn has_data(&self) -> bool {
        self.path.join("data").read_dir().is_ok_and(|mut d| d.next().is_some())
    }
}

/// Load all nodes from a project's nodes/ directory.
#[allow(dead_code)]
pub(crate) fn load_all_nodes(project_dir: &Path) -> Vec<Node> {
    let nodes_dir = project_dir.join("nodes");
    let Ok(entries) = std::fs::read_dir(&nodes_dir) else {
        return vec![];
    };
    let mut nodes = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            if let Ok(node) = Node::load(&entry.path()) {
                nodes.push(node);
            }
        }
    }
    nodes
}

#[allow(dead_code)]
fn now_iso() -> String {
    // Simple UTC timestamp without pulling in chrono
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", dur.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_load_node() {
        let dir = tempdir().unwrap();
        let nodes_dir = dir.path().join("nodes");
        std::fs::create_dir_all(&nodes_dir).unwrap();

        let meta = NodeMeta::new_root("Raw Flows");
        let node = Node::create(&nodes_dir, "raw-flows", meta).unwrap();
        assert_eq!(node.dir_name, "raw-flows");
        assert_eq!(node.meta.name, "Raw Flows");
        assert_eq!(node.meta.state, NodeState::Active);
        assert!(node.path.join("meta.toml").exists());
        assert!(node.path.join("views").exists());
        assert!(node.path.join("data").exists());

        // Reload
        let loaded = Node::load(&node.path).unwrap();
        assert_eq!(loaded.meta.name, "Raw Flows");
        assert_eq!(loaded.meta.state, NodeState::Active);
    }

    #[test]
    fn test_child_and_variant() {
        let dir = tempdir().unwrap();
        let nodes_dir = dir.path().join("nodes");
        std::fs::create_dir_all(&nodes_dir).unwrap();

        let root_meta = NodeMeta::new_root("Flows");
        Node::create(&nodes_dir, "flows", root_meta).unwrap();

        let child_meta = NodeMeta::new_child("TCP Only", "flows");
        let child = Node::create(&nodes_dir, "tcp-only", child_meta).unwrap();
        assert_eq!(child.meta.parent.as_deref(), Some("flows"));

        let var_meta = NodeMeta::new_variant("TCP v2", "flows", "tcp-only");
        let var = Node::create(&nodes_dir, "tcp-v2", var_meta).unwrap();
        assert_eq!(var.meta.variant_of.as_deref(), Some("tcp-only"));
    }

    #[test]
    fn test_load_all_nodes() {
        let dir = tempdir().unwrap();
        let nodes_dir = dir.path().join("nodes");
        std::fs::create_dir_all(&nodes_dir).unwrap();

        Node::create(&nodes_dir, "a", NodeMeta::new_root("A")).unwrap();
        Node::create(&nodes_dir, "b", NodeMeta::new_root("B")).unwrap();

        let all = load_all_nodes(dir.path());
        assert_eq!(all.len(), 2);
    }
}
