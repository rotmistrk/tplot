//! Lineage tree-table view — wraps TreeTableView<LineageData> for the left panel.

use txv_core::cursor::CursorRequest;
use txv_core::prelude::*;
use txv_widgets::TreeTableView;

use crate::lineage_data::LineageData;

pub(crate) struct LineageTreeView {
    pub(crate) inner: TreeTableView<LineageData>,
}

impl LineageTreeView {
    pub(crate) fn new(data: LineageData) -> Self {
        let mut view = TreeTableView::new(data, &[4, 8]);
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
