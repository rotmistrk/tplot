//! Workspace builder — constructs the initial tplot layout.

use std::path::Path;

use txv_widgets::tiled_workspace::types::{PanelConfig, PanelPosition, SplitNode};
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::lineage_data::LineageData;
use crate::slots::SlotId;
use crate::views::lineage_tree::LineageTreeView;
use crate::views::placeholder::PlaceholderView;
use crate::views::repl::ReplView;

/// Build the tplot workspace with 3 panels (left, center, tools).
pub fn build_workspace(_root_dir: &Path) -> TiledWorkspace {
    let configs = vec![
        PanelConfig::fixed("Lineage", PanelPosition::Left),
        PanelConfig::new("Main", PanelPosition::Center).with_splittable(),
        PanelConfig::new("Tools", PanelPosition::Right),
    ];
    let wide_layout = SplitNode::h(vec![
        (0.2, SplitNode::leaf(0)),
        (0.5, SplitNode::leaf(1)),
        (0.3, SplitNode::leaf(2)),
    ]);
    let narrow_layout = SplitNode::v(vec![
        (
            0.7,
            SplitNode::h(vec![(0.2, SplitNode::leaf(0)), (0.8, SplitNode::leaf(1))]),
        ),
        (0.3, SplitNode::leaf(2)),
    ]);

    let mut ws = TiledWorkspace::new(configs, wide_layout, narrow_layout, 300);
    ws.set_handle_keys(false);

    add_left_tabs(&mut ws, _root_dir);
    add_center_tabs(&mut ws);
    add_tools_tabs(&mut ws);

    ws.focus_panel(SlotId::Tools as usize);
    ws
}

fn add_left_tabs(ws: &mut TiledWorkspace, _root_dir: &Path) {
    let slot = SlotId::Left as usize;
    let lineage_data = LineageData::empty();
    let lineage_view = LineageTreeView::new(lineage_data);
    ws.insert_tab(slot, "Lineage", Box::new(lineage_view));
    insert(ws, slot, "Library", "Recipes & tools");
    insert(ws, slot, "Todo", "Task tracking");
    if let Some(panel) = ws.panel_mut(slot) {
        panel.set_active(0);
    }
}

fn add_center_tabs(ws: &mut TiledWorkspace) {
    let slot = SlotId::Center as usize;
    insert(ws, slot, "Welcome", "tplot — F1 for help, F4 to start typing commands");
}

fn add_tools_tabs(ws: &mut TiledWorkspace) {
    let slot = SlotId::Tools as usize;
    let repl = ReplView::new();
    ws.insert_tab(slot, "Tcl", Box::new(repl));
    insert(ws, slot, "Shell", "Terminal (not yet)");
    insert(ws, slot, "Messages", "Log output");
    if let Some(panel) = ws.panel_mut(slot) {
        panel.set_active(0);
    }
}

fn insert(ws: &mut TiledWorkspace, slot: usize, title: &str, desc: &str) {
    let view = PlaceholderView::new(desc);
    ws.insert_tab(slot, title, Box::new(view));
}
