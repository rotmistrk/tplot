//! Command editor view — uses txv-edit EditorView for multi-line command input.
//! Execution triggered via status bar bindings, not direct key intercept.

use txv_core::event::CommandId;
use txv_core::prelude::*;
use txv_edit::editor::EditorAction;
use txv_edit::view::{EditorView, EditorViewDelegate};

/// Base for editor execution commands.
const CM_EXEC_BASE: CommandId = txv_core::commands::CM_TXV_MAX + 200;

/// Execute current line.
pub const CM_EXEC_LINE: CommandId = CM_EXEC_BASE;
/// Execute visual selection.
pub const CM_EXEC_SELECTION: CommandId = CM_EXEC_BASE + 1;
/// Execute entire buffer.
pub const CM_EXEC_BUFFER: CommandId = CM_EXEC_BASE + 2;
/// Re-execute last command.
pub const CM_EXEC_LAST: CommandId = CM_EXEC_BASE + 3;
/// Trigger completion dropdown in editor.
pub const CM_EDITOR_COMPLETE: CommandId = CM_EXEC_BASE + 4;

/// Delegate that converts Ctrl-N completion into a command.
pub(crate) struct CmdDelegate;

impl EditorViewDelegate for CmdDelegate {
    fn on_action(&mut self, action: &EditorAction) -> bool {
        if matches!(action, EditorAction::LspCompletion) {
            return true; // handled — we emit CM_EDITOR_COMPLETE in on_action_post
        }
        false
    }

    fn on_action_post(&mut self, action: &EditorAction, _editor: &txv_edit::editor::Editor) {
        // We can't emit commands here since we don't have access to state.
        // Instead, mark a flag for the handle() override to pick up.
    }
}

pub struct CommandEditor {
    pub(crate) inner: EditorView<CmdDelegate>,
    /// Set when Ctrl-N triggers completion (picked up in handle).
    completion_requested: bool,
}

impl CommandEditor {
    pub fn new() -> Self {
        let mut editor = EditorView::with_delegate(CmdDelegate);
        editor.set_content("", "tcl");
        Self { inner: editor, completion_requested: false }
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

    /// Set the editor content (replaces buffer).
    pub fn set_content(&mut self, text: &str) {
        self.inner.set_content(text, "tcl");
    }

    /// Get the full editor content.
    pub fn content(&self) -> String {
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
        // Check for Ctrl-N to trigger completion
        if let txv_core::event::Event::Key(key) = event {
            if key.modifiers().ctrl() && key.code() == txv_core::event::KeyCode::Char('n') {
                let editor = self.inner.editor();
                let line = editor.cursor_line();
                let col = editor.cursor_col();
                let text = editor.buf().line(line).unwrap_or_default();
                let word_start = text[..col].rfind(|c: char| !c.is_alphanumeric() && c != '_').map(|i| i + 1).unwrap_or(0);
                let prefix = text[word_start..col].to_string();
                self.inner.put_command(CM_EDITOR_COMPLETE, Some(Box::new(prefix)));
                return txv_core::view::HandleResult::Consumed;
            }
        }
        self.inner.handle(event)
    }
}

/// Check if braces and quotes are balanced in Tcl input.
/// A line ending with unmatched `{` means the command continues.
fn is_tcl_complete(input: &str) -> bool {
    let mut brace_depth: i32 = 0;
    let mut in_quote = false;
    let mut prev_backslash = false;

    for ch in input.chars() {
        if prev_backslash {
            prev_backslash = false;
            continue;
        }
        match ch {
            '\\' => prev_backslash = true,
            '"' if brace_depth == 0 => in_quote = !in_quote,
            '{' if !in_quote => brace_depth += 1,
            '}' if !in_quote => brace_depth -= 1,
            _ => {}
        }
    }
    brace_depth <= 0 && !in_quote
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tcl_completeness() {
        assert!(!is_tcl_complete("sql {"));
        assert!(!is_tcl_complete("sql {\n  select 1"));
        assert!(is_tcl_complete("sql {\n  select 1\n}"));
        assert!(is_tcl_complete("sql {select 1}"));
        assert!(is_tcl_complete("select 1"));
    }
}
