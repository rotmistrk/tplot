//! Table view — displays query results as scrollable columns.

use txv_core::prelude::*;
use txv_core::view::HandleResult;

use crate::engine::QueryResult;

pub(crate) struct TableView {
    state: ViewState,
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<u16>,
    scroll_row: usize,
    scroll_col: usize,
    cursor_row: usize,
}

impl TableView {
    pub(crate) fn new(name: &str, result: QueryResult) -> Self {
        let col_widths = compute_col_widths(&result.columns, &result.rows);
        Self {
            state: ViewState::default(),
            name: name.to_string(),
            columns: result.columns,
            rows: result.rows,
            col_widths,
            scroll_row: 0,
            scroll_col: 0,
            cursor_row: 0,
        }
    }

    /// Update with new query results.
    #[allow(dead_code)]
    pub(crate) fn update(&mut self, result: QueryResult) {
        self.col_widths = compute_col_widths(&result.columns, &result.rows);
        self.columns = result.columns;
        self.rows = result.rows;
        self.scroll_row = 0;
        self.cursor_row = 0;
        self.state.mark_dirty();
    }

    fn visible_rows(&self) -> usize {
        let h = self.state.bounds().h() as usize;
        h.saturating_sub(2) // header + separator
    }

    fn handle_key(&mut self, ev: &Event) -> HandleResult {
        let Event::Key(key) = ev else {
            return HandleResult::Ignored;
        };
        use txv_core::event::KeyCode;
        match key.code() {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor_row + 1 < self.rows.len() {
                    self.cursor_row += 1;
                    self.ensure_visible();
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor_row = self.cursor_row.saturating_sub(1);
                self.ensure_visible();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::PageDown => {
                let page = self.visible_rows();
                self.cursor_row = (self.cursor_row + page).min(self.rows.len().saturating_sub(1));
                self.ensure_visible();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::PageUp => {
                let page = self.visible_rows();
                self.cursor_row = self.cursor_row.saturating_sub(page);
                self.ensure_visible();
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.scroll_col + 1 < self.columns.len() {
                    self.scroll_col += 1;
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.scroll_col = self.scroll_col.saturating_sub(1);
                self.state.mark_dirty();
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }

    fn ensure_visible(&mut self) {
        let vis = self.visible_rows();
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + vis {
            self.scroll_row = self.cursor_row - vis + 1;
        }
    }
}

impl View for TableView {
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
        let header_style = Style::new(txv_core::cell::Color::Ansi(14), txv_core::cell::Color::Reset);
        let cursor_style = Style::new(txv_core::cell::Color::Ansi(0), txv_core::cell::Color::Ansi(7));

        // Draw header.
        let header = format_row(&self.columns, &self.col_widths, self.scroll_col);
        buf.print_line(0, 0, &header, w, header_style);

        // Separator.
        let sep: String = "─".repeat(w as usize);
        buf.print_line(0, 1, &sep, w, style);

        // Data rows.
        let visible = h.saturating_sub(2);
        for i in 0..visible {
            let row_idx = self.scroll_row + i;
            if row_idx >= self.rows.len() {
                break;
            }
            let line = format_row(&self.rows[row_idx], &self.col_widths, self.scroll_col);
            let s = if row_idx == self.cursor_row {
                cursor_style
            } else {
                style
            };
            buf.print_line(0, (i + 2) as u16, &line, w, s);
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        self.handle_key(event)
    }
}

fn compute_col_widths(columns: &[String], rows: &[Vec<String>]) -> Vec<u16> {
    let mut widths: Vec<u16> = columns.iter().map(|c| c.len() as u16).collect();
    for row in rows.iter().take(100) {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len() as u16);
            }
        }
    }
    widths.iter_mut().for_each(|w| *w = (*w).clamp(3, 30));
    widths
}

fn format_row(cells: &[String], widths: &[u16], scroll_col: usize) -> String {
    let mut out = String::new();
    for (i, cell) in cells.iter().enumerate().skip(scroll_col) {
        let w = widths.get(i).copied().unwrap_or(10) as usize;
        let truncated: String = cell.chars().take(w).collect();
        out.push_str(&format!("{:<width$} │ ", truncated, width = w));
    }
    out
}
