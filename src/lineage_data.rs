//! Lineage tree-table data — backed by live NodeRegistry.

use txv_core::cell::Style;
use txv_widgets::tree_table_source::TreeTableSource;

use crate::live_node::{LiveNode, NodeKind};

/// Lineage tree-table data source, built from a snapshot of registry nodes.
pub(crate) struct LineageData {
    /// Flat list: (node_index, depth, expanded, is_link)
    entries: Vec<Entry>,
    visible: Vec<usize>,
    nodes: Vec<LiveNode>,
}

struct Entry {
    node_idx: usize,
    depth: usize,
    expanded: bool,
}

impl LineageData {
    pub(crate) fn from_nodes(nodes: Vec<LiveNode>) -> Self {
        let mut data = Self {
            entries: vec![],
            visible: vec![],
            nodes,
        };
        data.rebuild();
        data
    }

    #[allow(dead_code)]
    pub(crate) fn empty() -> Self {
        Self {
            entries: vec![],
            visible: vec![],
            nodes: vec![],
        }
    }

    /// Replace nodes and rebuild tree.
    pub(crate) fn update(&mut self, nodes: Vec<LiveNode>) {
        self.nodes = nodes;
        self.rebuild();
    }

    fn rebuild(&mut self) {
        self.entries.clear();
        // Find roots (no parent).
        let roots: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(i, _)| i)
            .collect();

        for root_idx in roots {
            self.add_subtree(root_idx, 0);
        }
        self.visible = (0..self.entries.len()).collect();
    }

    fn add_subtree(&mut self, node_idx: usize, depth: usize) {
        self.entries.push(Entry {
            node_idx,
            depth,
            expanded: true,
        });

        let name = self.nodes[node_idx].name.clone();
        let children: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.parent.as_deref() == Some(&name))
            .map(|(i, _)| i)
            .collect();

        for child_idx in children {
            self.add_subtree(child_idx, depth + 1);
        }
    }

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
                let has_children = self.entries.get(idx + 1).is_some_and(|e| e.depth > entry.depth);
                if has_children {
                    skip_below = Some(entry.depth);
                }
            }
        }
    }
}

impl TreeTableSource for LineageData {
    fn visible_count(&self) -> usize {
        self.visible.len()
    }

    fn label(&self, row: usize) -> &str {
        let entry = &self.entries[self.visible[row]];
        &self.nodes[entry.node_idx].name
    }

    fn depth(&self, row: usize) -> usize {
        self.entries[self.visible[row]].depth
    }

    fn is_expandable(&self, row: usize) -> bool {
        let idx = self.visible[row];
        let depth = self.entries[idx].depth;
        self.entries.get(idx + 1).is_some_and(|e| e.depth > depth)
    }

    fn is_expanded(&self, row: usize) -> bool {
        self.entries[self.visible[row]].expanded
    }

    fn toggle(&mut self, row: usize) {
        let idx = self.visible[row];
        self.entries[idx].expanded = !self.entries[idx].expanded;
        self.rebuild_visible();
    }

    fn style(&self, _row: usize) -> Style {
        Style::default()
    }

    fn column_count(&self) -> usize {
        1
    }

    fn cell(&self, row: usize, col: usize) -> &str {
        if col != 0 {
            return "";
        }
        let entry = &self.entries[self.visible[row]];
        let node = &self.nodes[entry.node_idx];
        match node.kind {
            NodeKind::Table => "[T]",
            NodeKind::Query => "[Q]",
            NodeKind::Plot => "[P]",
        }
    }
}
