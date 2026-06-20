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

    /// Get the complete command at cursor — uses Tcl parser to find command boundaries.
    /// Collects lines from cursor position until the Tcl parser accepts the input.
    pub fn current_command(&self) -> String {
        let editor = self.inner.editor();
        let buf = editor.buf();
        let cursor_line = editor.cursor_line();
        let line_count = buf.line_count();

        // Find start: scan backwards past empty/comment lines to find command start.
        let mut start = cursor_line;
        for i in (0..cursor_line).rev() {
            let l = buf.line(i).unwrap_or_default();
            let trimmed = l.trim();
            if trimmed.is_empty() || trimmed.starts_with("--") || trimmed.starts_with('#') {
                start = i + 1;
                break;
            }
            start = i;
        }

        // Collect lines from start, try parsing after each addition.
        let mut collected = String::new();
        for i in start..line_count {
            let l = buf.line(i).unwrap_or_default();
            let trimmed = l.trim();
            if i > start && trimmed.is_empty() && is_tcl_complete(&collected) {
                break;
            }
            if !collected.is_empty() {
                collected.push('\n');
            }
            collected.push_str(&l);
            // If we've passed the cursor and the command is complete, stop.
            if i >= cursor_line && is_tcl_complete(&collected) {
                break;
            }
        }
        collected
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
    delegate_view!(inner, override { title, handle, as_any_mut, group_state });

    fn title(&self) -> &str {
        "Cmd"
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn group_state(&self) -> Option<&txv_core::group::GroupState> {
        self.inner.group_state()
    }

    fn handle(&mut self, event: &txv_core::event::Event) -> txv_core::view::HandleResult {
        self.inner.handle(event)
    }
}

/// Check if a string is a complete Tcl command (braces/quotes balanced).
/// Uses rusticle's parser — if parse succeeds, the input is complete.
fn is_tcl_complete(input: &str) -> bool {
    use rusticle::parser::Parser;
    Parser::parse(input).is_ok()
}
