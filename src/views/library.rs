//! Library view — browse and load recipe scripts.

use txv_core::event::{CommandId, Event, KeyCode};
use txv_core::prelude::*;
use txv_core::view::HandleResult;

/// Emitted when user selects a recipe. Payload: recipe content (String).
pub(crate) const CM_RECIPE_LOAD: CommandId = txv_core::commands::CM_TXV_MAX + 300;

pub(crate) struct LibraryView {
    state: ViewState,
    recipes: Vec<Recipe>,
    cursor: usize,
    scroll: usize,
}

struct Recipe {
    name: String,
    description: String,
    content: String,
}

impl LibraryView {
    pub(crate) fn new() -> Self {
        let recipes = load_bundled_recipes();
        Self {
            state: ViewState::new(ViewOptions::default().with_focusable()),
            recipes,
            cursor: 0,
            scroll: 0,
        }
    }

    /// Also load project-local recipes from recipes/ dir.
    pub(crate) fn with_project_dir(project_dir: &std::path::Path) -> Self {
        let mut recipes = load_bundled_recipes();
        let recipe_dir = project_dir.join("recipes");
        if recipe_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&recipe_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("tcl") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                            let desc = content.lines().next().unwrap_or("").trim_start_matches("# ").to_string();
                            recipes.push(Recipe { name, description: desc, content });
                        }
                    }
                }
            }
        }
        Self {
            state: ViewState::new(ViewOptions::default().with_focusable()),
            recipes,
            cursor: 0,
            scroll: 0,
        }
    }
}

impl View for LibraryView {
    delegate_view_state!(state, override { title, draw, handle });

    fn title(&self) -> &str {
        "Library"
    }

    fn draw(&mut self) {
        let buf = self.state.buffer_mut();
        let w = buf.width() as usize;
        let h = buf.height() as usize;
        let normal = Style::default();
        let highlight = Style::new(Color::Rgb(255, 255, 255), Color::Rgb(60, 60, 100));
        let dim = Style::new(Color::Rgb(140, 140, 140), Style::default().bg());

        for row in 0..h {
            buf.hline(0, row as u16, w as u16, ' ', normal);
            let idx = self.scroll + row;
            if idx >= self.recipes.len() {
                continue;
            }
            let r = &self.recipes[idx];
            let style = if idx == self.cursor { highlight } else { normal };
            let label = format!(" {} ", r.name);
            buf.print(0, row as u16, &label, style);
            if w > label.len() + 2 {
                let desc: String = r.description.chars().take(w - label.len() - 1).collect();
                buf.print(label.len() as u16, row as u16, &desc, dim);
            }
        }
    }

    fn handle(&mut self, event: &Event) -> HandleResult {
        let Event::Key(key) = event else {
            return HandleResult::Ignored;
        };
        if self.recipes.is_empty() {
            return HandleResult::Ignored;
        }
        match key.code() {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor + 1 < self.recipes.len() {
                    self.cursor += 1;
                    let h = self.state.bounds().h() as usize;
                    if self.cursor >= self.scroll + h {
                        self.scroll = self.cursor - h + 1;
                    }
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    if self.cursor < self.scroll {
                        self.scroll = self.cursor;
                    }
                    self.state.mark_dirty();
                }
                HandleResult::Consumed
            }
            KeyCode::Enter | KeyCode::Right => {
                let content = self.recipes[self.cursor].content.clone();
                self.state.put_command(CM_RECIPE_LOAD, Some(Box::new(content)));
                HandleResult::Consumed
            }
            _ => HandleResult::Ignored,
        }
    }
}

fn load_bundled_recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            name: "ssh_auth_failures".into(),
            description: "SSH failed login attempts from auth.log".into(),
            content: include_str!("../../recipes/ssh_auth_failures.tcl").into(),
        },
        Recipe {
            name: "network_connections".into(),
            description: "Active network connections and listeners".into(),
            content: include_str!("../../recipes/network_connections.tcl").into(),
        },
        Recipe {
            name: "filesystem_stats".into(),
            description: "File sizes and directory analysis".into(),
            content: include_str!("../../recipes/filesystem_stats.tcl").into(),
        },
        Recipe {
            name: "syslog_events".into(),
            description: "System log events with severity".into(),
            content: include_str!("../../recipes/syslog_events.tcl").into(),
        },
        Recipe {
            name: "plotting_examples".into(),
            description: "Chart examples (bar, line)".into(),
            content: include_str!("../../recipes/plotting_examples.tcl").into(),
        },
    ]
}
