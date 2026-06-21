//! Lineage tree-table view — wraps TreeTableView<LineageData> for the left panel.

use txv_core::cursor::CursorRequest;
use txv_core::event::{CommandId, Event, KeyCode};
use txv_core::prelude::*;
use txv_core::view::HandleResult;
use txv_widgets::tree_table_source::TreeTableSource;
use txv_widgets::TreeTableView;

use crate::lineage_data::LineageData;

/// Emitted when user presses Enter on a lineage tree node. Payload: node name (String).
pub(crate) const CM_NODE_SELECT: CommandId = 901;
/// Emitted when user presses 'e' on a lineage tree node. Payload: node name (String).
pub(crate) const CM_NODE_EDIT: CommandId = 903;

pub(crate) struct LineageTreeView {
    pub(crate) inner: TreeTableView<LineageData>,
}

impl LineageTreeView {
    pub(crate) fn new(data: LineageData) -> Self {
        let mut view = TreeTableView::new(data, &[3]);
        view.set_show_connectors(true);
        Self { inner: view }
    }
}

impl View for LineageTreeView {
    delegate_view!(inner, override { title, cursor, handle, as_any_mut });

    fn title(&self) -> &str {
        "Lineage"
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn cursor(&self) -> Option<CursorRequest> {
        None
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        if let Event::Key(key) = event {
            let cursor = self.inner.cursor();
            let data = self.inner.data();
            if cursor < data.visible_count() {
                match key.code() {
                    KeyCode::Enter => {
                        let name = data.label(cursor).to_string();
                        self.inner.state_mut().put_command(CM_NODE_SELECT, Some(Box::new(name)));
                        return HandleResult::Consumed;
                    }
                    KeyCode::Char('e') => {
                        let name = data.label(cursor).to_string();
                        self.inner.state_mut().put_command(CM_NODE_EDIT, Some(Box::new(name)));
                        return HandleResult::Consumed;
                    }
                    _ => {}
                }
            }
        }
        self.inner.handle(event)
    }
}
