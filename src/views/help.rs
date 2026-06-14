//! Help view — shows command reference in a scrollable text area.

use txv_core::prelude::*;
use txv_widgets::TextArea;

use crate::help::help_text;

pub(crate) struct HelpView {
    inner: TextArea,
}

impl HelpView {
    pub(crate) fn new() -> Self {
        let mut ta = TextArea::new();
        ta.show_line_numbers(false);
        ta.set_content(&help_text());
        Self { inner: ta }
    }
}

impl View for HelpView {
    delegate_view!(inner, override { title });

    fn title(&self) -> &str {
        "Help"
    }
}
