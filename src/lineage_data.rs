//! Lineage tree data model — implements TreeData for the node DAG.

use std::path::{Path, PathBuf};

use txv_core::cell::Style;
use txv_widgets::tree_view::TreeData;

use crate::node::{self, Node, NodeState};

/// A flattened tree entry for display.
struct TreeEntry {
    node_idx: usize,
    depth: usize,
    expanded: bool,
    is_link: bool,
}

/// Lineage tree data source for TreeView.
pub(crate) struct LineageData {
    nodes: Vec<Node>,
    entries: Vec<TreeEntry>,
    visible: Vec<usize>,
    project_dir: PathBuf,
}

impl LineageData {
    pub(crate) fn new(project_dir: &Path) -> Self {
        let nodes = node::load_all_nodes(project_dir);
        let mut data = Self {
            nodes,
            entries: vec![],
            visible: vec![],
            project_dir: project_dir.to_path_buf(),
        };
        data.rebuild();
        data
    }

    /// Reload nodes from disk.
    #[allow(dead_code)]
    pub(crate) fn refresh(&mut self) {
        self.nodes = node::load_all_nodes(&self.project_dir);
        self.rebuild();
    }

    /// Rebuild flat entry list and visibility from nodes.
    fn rebuild(&mut self) {
        self.entries.clear();
        self.visible.clear();

        // Find root nodes (no parent)
        let roots: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.meta.parent.is_none())
            .map(|(i, _)| i)
            .collect();

        for &root_idx in &roots {
            self.add_subtree(root_idx, 0);
        }

        // All entries visible initially (expand later)
        self.visible = (0..self.entries.len()).collect();
    }

    fn add_subtree(&mut self, node_idx: usize, depth: usize) {
        self.entries.push(TreeEntry {
            node_idx,
            depth,
            expanded: true,
            is_link: false,
        });

        let dir_name = self.nodes[node_idx].dir_name.clone();
        // Find children (nodes whose parent matches this dir_name)
        let children: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.meta.parent.as_deref() == Some(&dir_name))
            .map(|(i, _)| i)
            .collect();

        for child_idx in children {
            self.add_subtree(child_idx, depth + 1);
        }

        // Add link entries for nodes that depend on this one (DAG edges)
        let link_count = self
            .nodes
            .iter()
            .filter(|n| n.meta.also_depends.contains(&dir_name))
            .count();

        for _ in 0..link_count {
            self.entries.push(TreeEntry {
                node_idx,
                depth: depth + 1,
                expanded: false,
                is_link: true,
            });
        }
    }

    #[allow(dead_code)]
    fn node_for_visible(&self, row: usize) -> Option<&Node> {
        let entry = self.entries.get(*self.visible.get(row)?)?;
        self.nodes.get(entry.node_idx)
    }

    fn state_icon(state: NodeState) -> &'static str {
        match state {
            NodeState::Active => "▸",
            NodeState::Materialized => "▸",
            NodeState::Frozen => "❄",
            NodeState::Ghost => "◇",
            NodeState::Stale => "⚠",
            NodeState::Running => "⟳",
            NodeState::Error => "✗",
        }
    }
}

impl TreeData for LineageData {
    fn root_count(&self) -> usize {
        self.entries.iter().filter(|e| e.depth == 0).count()
    }

    fn child_count(&self, id: usize) -> usize {
        let my_depth = match self.entries.get(id) {
            Some(e) => e.depth,
            None => return 0,
        };
        let mut count = 0;
        for e in self.entries.iter().skip(id + 1) {
            if e.depth <= my_depth {
                break;
            }
            if e.depth == my_depth + 1 {
                count += 1;
            }
        }
        count
    }

    fn label(&self, id: usize) -> &str {
        match self.entries.get(id) {
            Some(entry) => {
                if let Some(node) = self.nodes.get(entry.node_idx) {
                    &node.meta.name
                } else {
                    "?"
                }
            }
            None => "?",
        }
    }

    fn is_expandable(&self, id: usize) -> bool {
        self.child_count(id) > 0
    }

    fn is_expanded(&self, id: usize) -> bool {
        self.entries.get(id).is_some_and(|e| e.expanded)
    }

    fn toggle(&mut self, id: usize) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.expanded = !entry.expanded;
        }
        self.rebuild_visible();
    }

    fn depth(&self, id: usize) -> usize {
        self.entries.get(id).map_or(0, |e| e.depth)
    }

    fn visible_count(&self) -> usize {
        self.visible.len()
    }

    fn visible_id(&self, row: usize) -> usize {
        self.visible.get(row).copied().unwrap_or(0)
    }

    fn icon(&self, id: usize) -> Option<&str> {
        let entry = self.entries.get(id)?;
        if entry.is_link {
            return Some("⤴");
        }
        let node = self.nodes.get(entry.node_idx)?;
        Some(Self::state_icon(node.meta.state))
    }

    fn style(&self, id: usize) -> Style {
        let entry = match self.entries.get(id) {
            Some(e) => e,
            None => return Style::default(),
        };
        if entry.is_link {
            Style::default().with_fg(txv_core::cell::Color::Ansi(245))
        } else {
            Style::default()
        }
    }
}

impl LineageData {
    fn rebuild_visible(&mut self) {
        self.visible.clear();
        let mut skip_below: Option<usize> = None;
        for (idx, entry) in self.entries.iter().enumerate() {
            if let Some(skip_depth) = skip_below {
                if entry.depth > skip_depth {
                    continue;
                }
                skip_below = None;
            }
            self.visible.push(idx);
            if !entry.expanded {
                // Check if it has children (next entry has greater depth)
                let has_children = self.entries.get(idx + 1).is_some_and(|e| e.depth > entry.depth);
                if has_children {
                    skip_below = Some(entry.depth);
                }
            }
        }
    }
}
