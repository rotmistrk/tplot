//! PlaceholderView — simple text display for panels not yet implemented.

use txv_core::prelude::*;
use txv_widgets::TextArea;

pub(crate) struct PlaceholderView {
    inner: TextArea,
    label: String,
}

impl PlaceholderView {
    pub(crate) fn new(text: &str) -> Self {
        let mut ta = TextArea::new();
        ta.show_line_numbers(false);
        ta.set_content(text);
        Self {
            inner: ta,
            label: text.to_string(),
        }
    }
}

impl View for PlaceholderView {
    delegate_view!(inner, override { title });

    fn title(&self) -> &str {
        &self.label
    }
}
