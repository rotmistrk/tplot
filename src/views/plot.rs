//! Plot view — displays text-based charts in the center panel.

use txv_core::prelude::*;
use txv_core::view::HandleResult;

pub(crate) struct PlotView {
    state: ViewState,
    name: String,
    command: String,
    lines: Vec<String>,
    scroll: usize,
}

impl PlotView {
    pub(crate) fn new(name: &str, command: &str, lines: Vec<String>) -> Self {
        Self {
            state: ViewState::default(),
            name: name.to_string(),
            command: command.to_string(),
            lines,
            scroll: 0,
        }
    }
}

impl View for PlotView {
    delegate_view_state!(state, override { title, draw, handle });

    fn title(&self) -> &str {
        &self.name
    }

    fn draw(&mut self) {
        let buf = self.state.buffer_mut();
        let w = buf.width();
        let h = buf.height() as usize;
        if w == 0 || h == 0 {
            return;
        }

        let style = Style::default();
        let cmd_style = Style::new(txv_core::cell::Color::Ansi(245), txv_core::cell::Color::Reset);

        let mut y: usize = 0;

        // Command at top.
        if !self.command.is_empty() {
            buf.print_line(0, y as u16, &self.command, w, cmd_style);
            y += 1;
        }

        // Plot lines.
        let visible = h.saturating_sub(y);
        for i in 0..visible {
            let line_idx = self.scroll + i;
            if line_idx >= self.lines.len() {
                break;
            }
            buf.print_line(0, (y + i) as u16, &self.lines[line_idx], w, style);
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };
        use txv_core::event::KeyCode;
        match key.code() {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll + 1 < self.lines.len() {
                    self.scroll += 1;
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }
}
