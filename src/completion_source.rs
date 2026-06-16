//! Completion list source — implements DropdownSource for the completion popup.

use txv_widgets::dropdown_source::DropdownSource;

/// Simple list of completion strings for the dropdown menu.
pub(crate) struct CompletionListSource {
    items: Vec<String>,
}

impl CompletionListSource {
    pub(crate) fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}

impl DropdownSource for CompletionListSource {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn label(&self, idx: usize) -> &str {
        self.items.get(idx).map(|s| s.as_str()).unwrap_or("")
    }
}
