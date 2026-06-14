//! REPL view — interactive Tcl command line with output history.

use txv_core::cursor::{CursorRequest, CursorShape};
use txv_core::event::{CommandId, Event, KeyCode};
use txv_core::prelude::*;
use txv_core::view::HandleResult;

/// Command ID emitted when user presses Enter in the REPL.
pub(crate) const CM_REPL_SUBMIT: CommandId = 900;

pub(crate) struct ReplView {
    state: ViewState,
    lines: Vec<ReplLine>,
    input: String,
    cursor: usize,
    history: Vec<String>,
    hist_pos: Option<usize>,
    scroll: usize,
}

#[derive(Clone)]
#[allow(dead_code)]
enum ReplLine {
    Command(String),
    Output(String),
    Error(String),
}

impl ReplView {
    pub(crate) fn new() -> Self {
        Self {
            state: ViewState::default(),
            lines: vec![ReplLine::Output("tplot REPL — type commands, F1 for help".into())],
            input: String::new(),
            cursor: 0,
            history: Vec::new(),
            hist_pos: None,
            scroll: 0,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn push_command(&mut self, cmd: &str) {
        self.lines.push(ReplLine::Command(cmd.to_string()));
        self.auto_scroll();
    }

    #[allow(dead_code)]
    pub(crate) fn push_output(&mut self, text: &str) {
        if !text.is_empty() {
            self.lines.push(ReplLine::Output(text.to_string()));
            self.auto_scroll();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn push_error(&mut self, text: &str) {
        self.lines.push(ReplLine::Error(text.to_string()));
        self.auto_scroll();
    }

    #[allow(dead_code)]
    pub(crate) fn take_input(&mut self) -> String {
        let text = self.input.clone();
        if !text.is_empty() {
            self.history.push(text.clone());
        }
        self.input.clear();
        self.cursor = 0;
        self.hist_pos = None;
        text
    }

    fn auto_scroll(&mut self) {
        let h = self.state.bounds().h() as usize;
        let visible = h.saturating_sub(1);
        if self.lines.len() > visible {
            self.scroll = self.lines.len() - visible;
        }
    }

    fn handle_key(&mut self, ev: &Event) -> HandleResult {
        let Event::Key(key) = ev else {
            return HandleResult::Ignored;
        };
        let code = key.code();
        match code {
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    self.state.put_command(CM_REPL_SUBMIT, None);
                }
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Char(ch) => {
                self.input.insert(self.cursor, ch);
                self.cursor += 1;
                self.hist_pos = None;
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::End => {
                self.cursor = self.input.len();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Up => {
                self.history_prev();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Down => {
                self.history_next();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }

    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let pos = match self.hist_pos {
            Some(p) => p.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.hist_pos = Some(pos);
        self.input = self.history[pos].clone();
        self.cursor = self.input.len();
    }

    fn history_next(&mut self) {
        match self.hist_pos {
            Some(p) if p + 1 < self.history.len() => {
                self.hist_pos = Some(p + 1);
                self.input = self.history[p + 1].clone();
                self.cursor = self.input.len();
            }
            _ => {
                self.hist_pos = None;
                self.input.clear();
                self.cursor = 0;
            }
        }
    }
}

impl View for ReplView {
    delegate_view_state!(state, override { title, draw, handle, cursor });

    fn title(&self) -> &str {
        "Tcl"
    }

    fn cursor(&self) -> Option<CursorRequest> {
        let h = self.state.bounds().h();
        if h == 0 {
            return None;
        }
        let col = (self.cursor as u16) + 7;
        Some(CursorRequest::new(col, h - 1, CursorShape::Bar))
    }

    fn draw(&mut self) {
        let buf = self.state.buffer_mut();
        let w = buf.width();
        let h = buf.height() as usize;
        if w == 0 || h == 0 {
            return;
        }

        let output_rows = h.saturating_sub(1);
        let style = Style::default();
        let err_style = Style::new(txv_core::cell::Color::Ansi(9), txv_core::cell::Color::Reset);

        for row in 0..output_rows {
            let line_idx = self.scroll + row;
            if line_idx < self.lines.len() {
                let (text, s) = match &self.lines[line_idx] {
                    ReplLine::Command(t) => (format!("» {t}"), style),
                    ReplLine::Output(t) => (t.clone(), style),
                    ReplLine::Error(t) => (format!("✗ {t}"), err_style),
                };
                buf.print_line(0, row as u16, &text, w, s);
            }
        }

        let prompt = format!("tplot> {}", self.input);
        buf.print_line(0, (h - 1) as u16, &prompt, w, style);
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.handle_key(event)
    }
}
