//! Live node — in-memory representation of a lineage tree entry.
//! Nodes are created automatically from commands and persisted as node.tcl files.

use std::path::Path;
use std::time::SystemTime;

/// Type of node in the lineage tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// Materialized table (into, CREATE TABLE) — data in DuckDB.
    Table,
    /// Query/view (sql SELECT) — re-runnable, not materialized.
    Query,
    /// Plot output.
    Plot,
}

/// A live node in the lineage tree.
#[derive(Clone, Debug)]
pub(crate) struct LiveNode {
    pub(crate) name: String,
    pub(crate) kind: NodeKind,
    pub(crate) parent: Option<String>,
    pub(crate) command: String,
    pub(crate) query_text: String,
    pub(crate) created: SystemTime,
    pub(crate) last_run: Option<SystemTime>,
    pub(crate) run_secs: Option<f64>,
    pub(crate) row_count: Option<u64>,
    pub(crate) note: String,
}

impl LiveNode {
    pub(crate) fn new(name: &str, kind: NodeKind, parent: Option<&str>, command: &str) -> Self {
        Self {
            name: name.to_string(),
            kind,
            parent: parent.map(String::from),
            command: command.to_string(),
            created: SystemTime::now(),
            last_run: Some(SystemTime::now()),
            run_secs: None,
            row_count: None,
            note: String::new(),
            query_text: String::new(),
        }
    }

    /// The executable SQL query for this node (extracted at creation time).
    pub(crate) fn query(&self) -> &str {
        &self.query_text
    }

    /// Set the query text explicitly.
    pub(crate) fn set_query(&mut self, q: &str) {
        self.query_text = q.to_string();
    }

    /// Serialize to node.tcl content.
    pub(crate) fn to_script(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# node: {}\n", self.name));
        if let Some(ref p) = self.parent {
            out.push_str(&format!("# parent: {p}\n"));
        } else {
            out.push_str("# parent: (root)\n");
        }
        out.push_str(&format!("# kind: {:?}\n", self.kind));
        out.push_str(&format!("# created: {}\n", format_time(self.created)));
        if let Some(t) = self.last_run {
            let dur = self.run_secs.map(|s| format!(" ({s:.2}s)")).unwrap_or_default();
            out.push_str(&format!("# last_run: {}{dur}\n", format_time(t)));
        }
        if let Some(rows) = self.row_count {
            out.push_str(&format!("# rows: {rows}\n"));
        }
        if !self.note.is_empty() {
            out.push_str(&format!("# note: {}\n", self.note));
        }
        out.push('\n');
        out.push_str(&self.command);
        out.push('\n');
        out
    }

    /// Parse a node.tcl file back into a LiveNode.
    pub(crate) fn from_script(content: &str) -> Option<Self> {
        let mut name = String::new();
        let mut parent: Option<String> = None;
        let mut kind = NodeKind::Query;
        let mut note = String::new();
        let mut row_count = None;
        let mut command_lines = Vec::new();

        for line in content.lines() {
            if let Some(val) = line.strip_prefix("# node: ") {
                name = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("# parent: ") {
                let v = val.trim();
                parent = if v == "(root)" {
                    None
                } else {
                    Some(v.to_string())
                };
            } else if let Some(val) = line.strip_prefix("# kind: ") {
                kind = match val.trim() {
                    "Table" => NodeKind::Table,
                    "Plot" => NodeKind::Plot,
                    _ => NodeKind::Query,
                };
            } else if let Some(val) = line.strip_prefix("# note: ") {
                note = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("# rows: ") {
                row_count = val.trim().parse().ok();
            } else if !line.starts_with('#') && !line.is_empty() {
                command_lines.push(line.to_string());
            }
        }

        if name.is_empty() {
            return None;
        }

        let mut node = LiveNode::new(&name, kind, parent.as_deref(), &command_lines.join("\n"));
        node.row_count = row_count;
        node.note = note;
        Some(node)
    }
}

/// Save a node to disk as node.tcl.
pub(crate) fn save_node(nodes_dir: &Path, path_segments: &[&str], node: &LiveNode) -> Result<(), String> {
    let mut dir = nodes_dir.to_path_buf();
    for seg in path_segments {
        dir.push(seg);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let file = dir.join("node.tcl");
    std::fs::write(&file, node.to_script()).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// Load all nodes from disk (recursive walk).
pub(crate) fn load_nodes_from_disk(nodes_dir: &Path) -> Vec<LiveNode> {
    let mut nodes = Vec::new();
    walk_nodes(nodes_dir, &mut nodes);
    nodes
}

fn walk_nodes(dir: &Path, out: &mut Vec<LiveNode>) {
    let node_file = dir.join("node.tcl");
    if node_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&node_file) {
            if let Some(node) = LiveNode::from_script(&content) {
                out.push(node);
            }
        }
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            walk_nodes(&entry.path(), out);
        }
    }
}

fn format_time(t: SystemTime) -> String {
    let secs = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs();
    // Simple ISO-ish format without chrono
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let y = 1970 + days / 365; // approximate
    format!("{y}-xx-xx {h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut node = LiveNode::new(
            "by_user",
            NodeKind::Query,
            Some("auth"),
            "sql -name by_user {SELECT username, count(*) FROM auth GROUP BY 1}",
        );
        node.row_count = Some(3);
        node.note = "Top attackers".to_string();

        let script = node.to_script();
        assert!(script.contains("# node: by_user"));
        assert!(script.contains("# parent: auth"));
        assert!(script.contains("# rows: 3"));
        assert!(script.contains("sql -name by_user"));

        let parsed = LiveNode::from_script(&script).unwrap();
        assert_eq!(parsed.name, "by_user");
        assert_eq!(parsed.parent.as_deref(), Some("auth"));
        assert_eq!(parsed.kind, NodeKind::Query);
        assert_eq!(parsed.row_count, Some(3));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let nodes_dir = dir.path().join("nodes");

        let node = LiveNode::new("auth", NodeKind::Table, None, "into auth -file auth.csv");
        save_node(&nodes_dir, &["000"], &node).unwrap();

        let child = LiveNode::new("by_user", NodeKind::Query, Some("auth"), "sql {SELECT ...}");
        save_node(&nodes_dir, &["000", "000"], &child).unwrap();

        let loaded = load_nodes_from_disk(&nodes_dir);
        assert_eq!(loaded.len(), 2);
    }
}
