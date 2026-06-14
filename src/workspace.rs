//! Workspace builder — constructs the initial tplot layout.

use std::path::Path;

use txv_widgets::tiled_workspace::types::{PanelConfig, PanelPosition, SplitNode};
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::lineage_data::LineageData;
use crate::slots::SlotId;
use crate::views::lineage_tree::LineageTreeView;
use crate::views::placeholder::PlaceholderView;

/// Build the tplot workspace with 4 panels.
pub(crate) fn build_workspace(root_dir: &Path) -> TiledWorkspace {
    let configs = vec![
        PanelConfig::fixed("Lineage", PanelPosition::Left),
        PanelConfig::new("Main", PanelPosition::Center).with_splittable(),
        PanelConfig::new("Tools", PanelPosition::Right),
        PanelConfig::fixed("Command", PanelPosition::Bottom),
    ];
    let wide_layout = SplitNode::h(vec![
        (0.2, SplitNode::leaf(0)),
        (
            0.8,
            SplitNode::v(vec![
                (
                    0.7,
                    SplitNode::h(vec![(0.6, SplitNode::leaf(1)), (0.4, SplitNode::leaf(2))]),
                ),
                (0.3, SplitNode::leaf(3)),
            ]),
        ),
    ]);
    let narrow_layout = SplitNode::v(vec![
        (
            0.7,
            SplitNode::h(vec![(0.2, SplitNode::leaf(0)), (0.8, SplitNode::leaf(1))]),
        ),
        (0.15, SplitNode::leaf(2)),
        (0.15, SplitNode::leaf(3)),
    ]);

    let mut ws = TiledWorkspace::new(configs, wide_layout, narrow_layout, 300);
    ws.set_handle_keys(false);

    add_left_tabs(&mut ws, root_dir);
    add_center_tabs(&mut ws);
    add_tools_tabs(&mut ws);
    add_bottom_tabs(&mut ws);

    ws.focus_panel(SlotId::Center as usize);
    ws
}

fn add_left_tabs(ws: &mut TiledWorkspace, root_dir: &Path) {
    let slot = SlotId::Left as usize;

    let lineage_data = LineageData::new(root_dir);
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
    insert(ws, slot, "Welcome", "tplot — terminal data analysis");
}

fn add_tools_tabs(ws: &mut TiledWorkspace) {
    let slot = SlotId::Tools as usize;
    insert(ws, slot, "Shell", "Terminal");
    insert(ws, slot, "Messages", "Log output");
}

fn add_bottom_tabs(ws: &mut TiledWorkspace) {
    let slot = SlotId::Bottom as usize;
    insert(ws, slot, "Command", "Command line");
}

fn insert(ws: &mut TiledWorkspace, slot: usize, title: &str, desc: &str) {
    let view = PlaceholderView::new(desc);
    ws.insert_tab(slot, title, Box::new(view));
}
