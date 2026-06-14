//! Lineage tree-table data — implements TreeTableSource for the node DAG.

use std::path::{Path, PathBuf};

use txv_core::cell::Style;
use txv_widgets::tree_table_source::TreeTableSource;

use crate::node::{self, Node, NodeState};

/// Column indices for the tree-table.
const COL_STATUS: usize = 0;
const COL_TIME: usize = 1;
const COL_SIZE: usize = 2;

/// A flattened tree entry for display.
struct TreeEntry {
    node_idx: usize,
    depth: usize,
    expanded: bool,
    is_link: bool,
}

/// Lineage tree-table data source.
pub(crate) struct LineageData {
    nodes: Vec<Node>,
    entries: Vec<TreeEntry>,
    visible: Vec<usize>,
    project_dir: PathBuf,
    // Column visibility (user can toggle).
    show_status: bool,
    show_time: bool,
    show_size: bool,
    // Cached cell strings (regenerated on rebuild).
    cell_cache: Vec<[String; 3]>,
}

impl LineageData {
    pub(crate) fn new(project_dir: &Path) -> Self {
        let nodes = node::load_all_nodes(project_dir);
        let mut data = Self {
            nodes,
            entries: vec![],
            visible: vec![],
            project_dir: project_dir.to_path_buf(),
            show_status: true,
            show_time: true,
            show_size: true,
            cell_cache: vec![],
        };
        data.rebuild();
        data
    }

    #[allow(dead_code)]
    pub(crate) fn refresh(&mut self) {
        self.nodes = node::load_all_nodes(&self.project_dir);
        self.rebuild();
    }

    fn rebuild(&mut self) {
        self.entries.clear();
        self.visible.clear();
        self.cell_cache.clear();

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
        self.visible = (0..self.entries.len()).collect();
        self.rebuild_cells();
    }

    fn add_subtree(&mut self, node_idx: usize, depth: usize) {
        self.entries.push(TreeEntry {
            node_idx,
            depth,
            expanded: true,
            is_link: false,
        });

        let dir_name = self.nodes[node_idx].dir_name.clone();
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

    fn rebuild_cells(&mut self) {
        self.cell_cache.clear();
        for entry in &self.entries {
            let node = &self.nodes[entry.node_idx];
            let status = if entry.is_link {
                "⤴".to_string()
            } else {
                state_icon(node.meta.state).to_string()
            };
            let time = format_time(node);
            let size = format_size(node.meta.size.data_bytes);
            self.cell_cache.push([status, time, size]);
        }
    }
}

impl TreeTableSource for LineageData {
    fn visible_count(&self) -> usize {
        self.visible.len()
    }

    fn label(&self, row: usize) -> &str {
        let entry_idx = self.visible[row];
        let entry = &self.entries[entry_idx];
        &self.nodes[entry.node_idx].meta.name
    }

    fn depth(&self, row: usize) -> usize {
        self.entries[self.visible[row]].depth
    }

    fn is_expandable(&self, row: usize) -> bool {
        let entry_idx = self.visible[row];
        let depth = self.entries[entry_idx].depth;
        self.entries.get(entry_idx + 1).is_some_and(|e| e.depth > depth)
    }

    fn is_expanded(&self, row: usize) -> bool {
        self.entries[self.visible[row]].expanded
    }

    fn toggle(&mut self, row: usize) {
        let entry_idx = self.visible[row];
        self.entries[entry_idx].expanded = !self.entries[entry_idx].expanded;
        self.rebuild_visible();
    }

    fn style(&self, row: usize) -> Style {
        let entry = &self.entries[self.visible[row]];
        if entry.is_link {
            Style::default().with_fg(txv_core::cell::Color::Ansi(245))
        } else {
            Style::default()
        }
    }

    fn column_count(&self) -> usize {
        let mut n = 0;
        if self.show_status {
            n += 1;
        }
        if self.show_time {
            n += 1;
        }
        if self.show_size {
            n += 1;
        }
        n
    }

    fn cell(&self, row: usize, col: usize) -> &str {
        let entry_idx = self.visible[row];
        let cells = &self.cell_cache[entry_idx];
        let real_col = self.map_col(col);
        &cells[real_col]
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
                let has_children = self.entries.get(idx + 1).is_some_and(|e| e.depth > entry.depth);
                if has_children {
                    skip_below = Some(entry.depth);
                }
            }
        }
    }

    /// Map visible column index to internal cell array index.
    fn map_col(&self, col: usize) -> usize {
        let mut remaining = col;
        if self.show_status {
            if remaining == 0 {
                return COL_STATUS;
            }
            remaining -= 1;
        }
        if self.show_time {
            if remaining == 0 {
                return COL_TIME;
            }
            remaining -= 1;
        }
        if self.show_size && remaining == 0 {
            return COL_SIZE;
        }
        COL_STATUS // fallback
    }
}

fn state_icon(state: NodeState) -> &'static str {
    match state {
        NodeState::Active => "▸",
        NodeState::Materialized => "✓",
        NodeState::Frozen => "❄",
        NodeState::Ghost => "◇",
        NodeState::Stale => "⚠",
        NodeState::Running => ">",
        NodeState::Error => "✗",
    }
}

fn format_time(node: &Node) -> String {
    let secs = node.meta.timing.last_run_secs;
    if secs <= 0.0 {
        return String::new();
    }
    let h = (secs / 3600.0) as u32;
    let m = ((secs % 3600.0) / 60.0) as u32;
    let s = (secs % 60.0) as u32;
    format!("{h:02}:{m:02}:{s:02}..")
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return String::new();
    }
    if bytes < 1024 {
        return format!("{bytes}B");
    }
    if bytes < 1024 * 1024 {
        return format!("{}KB", bytes / 1024);
    }
    if bytes < 1024 * 1024 * 1024 {
        return format!("{}MB", bytes / (1024 * 1024));
    }
    format!("{}GB", bytes / (1024 * 1024 * 1024))
}
