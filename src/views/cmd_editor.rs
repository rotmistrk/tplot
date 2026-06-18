//! Command editor view — uses txv-edit EditorView for multi-line command input.
//! Execution triggered via status bar bindings, not direct key intercept.

use txv_core::event::CommandId;
use txv_core::prelude::*;
use txv_edit::view::EditorView;

/// Execute current line.
pub const CM_EXEC_LINE: CommandId = 910;
/// Execute visual selection.
pub const CM_EXEC_SELECTION: CommandId = 911;
/// Execute entire buffer.
pub const CM_EXEC_BUFFER: CommandId = 912;
/// Re-execute last command.
pub const CM_EXEC_LAST: CommandId = 913;

pub struct CommandEditor {
    inner: EditorView,
}

impl CommandEditor {
    pub fn new() -> Self {
        let editor = EditorView::from_text("");
        Self { inner: editor }
    }

    /// Get the current line text.
    pub fn current_line(&self) -> String {
        let editor = self.inner.editor();
        let line = editor.cursor_line();
        editor.buf().line(line).unwrap_or_default()
    }

    /// Get full buffer content.
    pub fn buffer_content(&self) -> String {
        self.inner.content()
    }
}

impl Default for CommandEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl View for CommandEditor {
    delegate_view!(inner, override { title, handle, as_any_mut });

    fn title(&self) -> &str {
        "Cmd"
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn handle(&mut self, event: &txv_core::event::Event) -> txv_core::view::HandleResult {
        use txv_core::event::Event;
        use txv_core::view::HandleResult;

        if let Event::Paste(text) = event {
            let editor = self.inner.editor_mut();
            let offset = editor
                .buf()
                .line_col_to_offset(editor.cursor_line(), editor.cursor_col())
                .unwrap_or(0);
            editor.buf().insert(offset, text);
            self.inner.mark_dirty();
            return HandleResult::Consumed;
        }
        self.inner.handle(event)
    }
}
