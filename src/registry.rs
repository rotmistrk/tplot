//! Node registry — manages the live lineage tree, auto-creates nodes from commands.

use std::path::{Path, PathBuf};

use crate::live_node::{self, LiveNode, NodeKind};

/// Manages the in-memory lineage tree.
pub(crate) struct NodeRegistry {
    nodes: Vec<LiveNode>,
    nodes_dir: PathBuf,
    next_root: u32,
}

impl NodeRegistry {
    pub(crate) fn new(project_dir: &Path) -> Self {
        let nodes_dir = project_dir.join("nodes");
        let nodes = live_node::load_nodes_from_disk(&nodes_dir);
        let next_root = Self::compute_next_root(&nodes_dir);
        Self {
            nodes,
            nodes_dir,
            next_root,
        }
    }

    /// Get all nodes (for tree display).
    pub(crate) fn nodes(&self) -> &[LiveNode] {
        &self.nodes
    }

    /// Register a materialized table (from `into` or `CREATE TABLE`).
    pub(crate) fn add_table(&mut self, name: &str, command: &str, row_count: Option<u64>) {
        if let Some(existing) = self.nodes.iter_mut().find(|n| n.name == name) {
            existing.command = command.to_string();
            existing.row_count = row_count;
            existing.last_run = Some(std::time::SystemTime::now());
            return;
        }
        let mut node = LiveNode::new(name, NodeKind::Table, None, command);
        node.row_count = row_count;
        self.persist_node_root(&node);
        self.nodes.push(node);
    }

    /// Register a query (from `sql`). Parent detected from FROM clause.
    pub(crate) fn add_query(&mut self, name: &str, command: &str, parent: Option<&str>, row_count: Option<u64>) {
        if let Some(existing) = self.nodes.iter_mut().find(|n| n.name == name) {
            existing.command = command.to_string();
            existing.row_count = row_count;
            existing.last_run = Some(std::time::SystemTime::now());
            return;
        }
        let mut node = LiveNode::new(name, NodeKind::Query, parent, command);
        node.row_count = row_count;
        self.persist_node_child(&node);
        self.nodes.push(node);
    }

    fn persist_node_root(&mut self, node: &LiveNode) {
        let seg = format!("{:03}", self.next_root);
        self.next_root += 1;
        let _ = live_node::save_node(&self.nodes_dir, &[&seg], node);
    }

    fn persist_node_child(&self, node: &LiveNode) {
        // Find parent's directory index.
        let parent_name = node.parent.as_deref().unwrap_or("");
        let parent_idx = self
            .nodes
            .iter()
            .filter(|n| n.parent.is_none())
            .position(|n| n.name == parent_name);

        if let Some(idx) = parent_idx {
            let parent_seg = format!("{idx:03}");
            let child_count = self
                .nodes
                .iter()
                .filter(|n| n.parent.as_deref() == Some(parent_name))
                .count();
            let child_seg = format!("{child_count:03}");
            let _ = live_node::save_node(&self.nodes_dir, &[&parent_seg, &child_seg], node);
        } else {
            // Parent not found as root — save as root
            let seg = format!("{:03}", self.next_root);
            let _ = live_node::save_node(&self.nodes_dir, &[&seg], node);
        }
    }

    fn compute_next_root(nodes_dir: &Path) -> u32 {
        let Ok(entries) = std::fs::read_dir(nodes_dir) else {
            return 0;
        };
        let mut max = 0u32;
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Ok(n) = name.parse::<u32>() {
                    max = max.max(n + 1);
                }
            }
        }
        max
    }
}

/// Try to detect the primary table from a SQL query (simple FROM clause parsing).
pub(crate) fn detect_parent_table(sql: &str) -> Option<String> {
    let upper = sql.to_uppercase();
    let from_pos = upper.find(" FROM ")?;
    let after_from = &sql[from_pos + 6..];
    let table = after_from
        .split(|c: char| c.is_whitespace() || c == ',' || c == ')' || c == ';')
        .next()?
        .trim_matches('"');
    if table.is_empty() || table.starts_with('(') {
        return None;
    }
    Some(table.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_parent() {
        assert_eq!(detect_parent_table("SELECT * FROM auth"), Some("auth".into()));
        assert_eq!(detect_parent_table("SELECT x FROM auth WHERE y=1"), Some("auth".into()));
        assert_eq!(detect_parent_table("SELECT 1"), None);
    }

    #[test]
    fn test_registry_add() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = NodeRegistry::new(dir.path());
        reg.add_table("auth", "into auth -file x.csv", Some(100));
        reg.add_query("by_user", "sql {SELECT ...}", Some("auth"), Some(3));

        assert_eq!(reg.nodes().len(), 2);
        assert_eq!(reg.nodes()[0].name, "auth");
        assert_eq!(reg.nodes()[1].parent.as_deref(), Some("auth"));
    }
}
