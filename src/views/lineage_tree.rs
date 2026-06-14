//! Lineage tree view — wraps TreeView<LineageData> for the left panel.

use txv_core::cursor::CursorRequest;
use txv_core::prelude::*;
use txv_widgets::TreeView;

use crate::lineage_data::LineageData;

pub(crate) struct LineageTreeView {
    inner: TreeView<LineageData>,
}

impl LineageTreeView {
    pub(crate) fn new(data: LineageData) -> Self {
        let mut view = TreeView::new(data);
        view.set_show_connectors(true);
        Self { inner: view }
    }
}

impl View for LineageTreeView {
    delegate_view!(inner, override { title, cursor });

    fn title(&self) -> &str {
        "Lineage"
    }

    fn cursor(&self) -> Option<CursorRequest> {
        None
    }
}
