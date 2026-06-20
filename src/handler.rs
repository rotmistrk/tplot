//! Command handler — dispatches commands from the TUI event loop.

use txv_core::program::CommandContext;
use txv_widgets::dropdown_menu::{CM_DROPDOWN_CANCELLED, CM_DROPDOWN_DONE};
use txv_widgets::sidekick::CM_SIDEKICK_RESULT;
use txv_widgets::tiled_workspace::TiledWorkspace;
use txv_widgets::CM_STATUS_MESSAGE;

use crate::app::AppState;
use crate::node_behavior::NodeResult;
use crate::registry;
use crate::scripting::ScriptCommand;
use crate::slots::SlotId;
use crate::sql_analysis;
use crate::status::{CM_APP_QUIT, CM_SHOW_HELP};
use crate::views::cmd_editor::{CommandEditor, CM_EXEC_BUFFER, CM_EXEC_LINE};
use crate::views::help::HelpView;
use crate::views::lineage_tree::{LineageTreeView, CM_NODE_SELECT};
use crate::views::plot::PlotView;
use crate::views::repl::{ReplView, CM_REPL_SUBMIT, CM_REPL_TAB};
use crate::views::table::TableView;

/// Refresh lineage tree on startup (called from main before event loop).
pub fn initial_refresh(desktop: &mut dyn txv_core::prelude::View, registry: &registry::Registry) {
    refresh_lineage_tree(desktop, registry);
    let count = registry.nodes().len();
    if count > 0 {
        log::info!("Loaded {count} nodes from disk");
    }
}

pub fn handle_command(ctx: &mut CommandContext, state: &mut AppState) {
    match ctx.command() {
        CM_REPL_SUBMIT => handle_repl_submit(ctx, state),
        CM_REPL_TAB => handle_repl_tab(ctx, state),
        CM_EXEC_LINE => handle_exec_line(ctx, state),
        CM_EXEC_BUFFER => handle_exec_buffer(ctx, state),
        CM_NODE_SELECT => handle_node_select(ctx, state),
        CM_SIDEKICK_RESULT => {
            // User picked from dropdown — apply completion (String payload).
            if let Some(text) = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()) {
                let text = text.clone();
                if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
                    repl.apply_completion(&text);
                    repl.sidekick_visible = false;
                }
            }
        }
        CM_DROPDOWN_DONE => {
            // Dropdown confirmed selection (usize index payload).
            // Re-run completion to get the items, pick by index.
            if let Some(&idx) = ctx.data().as_ref().and_then(|d| d.downcast_ref::<usize>()) {
                let input = {
                    let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
                        return;
                    };
                    repl.current_input().to_string()
                };
                let completions = crate::completions::complete(&input, state.engine(), &state.registry);
                if let Some(c) = completions.get(idx) {
                    let text = c.text.clone();
                    if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
                        repl.apply_completion(&text);
                        repl.sidekick_visible = false;
                    }
                }
            }
        }
        CM_DROPDOWN_CANCELLED => {
            if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
                repl.sidekick_visible = false;
            }
        }
        CM_APP_QUIT => {
            ctx.sink().push_command(txv_core::commands::CM_QUIT, None);
        }
        CM_SHOW_HELP => {
            if let Some(ws) = ctx
                .desktop_mut()
                .as_any_mut()
                .and_then(|a| a.downcast_mut::<TiledWorkspace>())
            {
                ws.insert_tab(SlotId::Center as usize, "Help", Box::new(HelpView::new()));
            }
        }
        _ => {}
    }
}

fn handle_repl_tab(ctx: &mut CommandContext, state: &mut AppState) {
    let input = {
        let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
            return;
        };
        repl.current_input().to_string()
    };
    let completions = crate::completions::complete(&input, state.engine(), &state.registry);
    if completions.len() == 1 {
        let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
            return;
        };
        repl.apply_completion(&completions[0].text);
    } else if completions.len() > 1 {
        let texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        let prefix = common_prefix(&texts);
        let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
            return;
        };
        if prefix.len() > input.split_whitespace().last().map_or(0, |w| w.len()) {
            repl.apply_completion(&prefix);
        }
        // Show dropdown with all options.
        let items: Vec<String> = completions.iter().map(|c| c.text.clone()).collect();
        repl.show_completion_dropdown(items);
    }
}

/// Push a status bar message (visible to user as popup).
fn status_msg(ctx: &CommandContext, text: &str) {
    use txv_core::message::{Message, MsgLevel};
    let msg = Message::new(MsgLevel::Info, "tplot", text.to_string());
    ctx.sink().push_command(CM_STATUS_MESSAGE, Some(Box::new(msg)));
}

fn status_err(ctx: &CommandContext, text: &str) {
    use txv_core::message::{Message, MsgLevel};
    let msg = Message::new(MsgLevel::Error, "tplot", text.to_string());
    ctx.sink().push_command(CM_STATUS_MESSAGE, Some(Box::new(msg)));
}

fn common_prefix(strings: &[&str]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = strings[0];
    let mut len = first.len();
    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, (a, b)) in first.chars().zip(s.chars()).enumerate() {
            if a != b {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
}

fn handle_exec_line(ctx: &mut CommandContext, state: &mut AppState) {
    let text = {
        let Some(editor) = find_cmd_editor(ctx.desktop_mut()) else {
            status_err(ctx, "cmd editor not found");
            return;
        };
        editor.current_command()
    };
    if text.trim().is_empty() || text.trim().starts_with("--") || text.trim().starts_with('#') {
        return;
    }
    exec_text(ctx, state, text.trim());
}

fn handle_exec_buffer(ctx: &mut CommandContext, state: &mut AppState) {
    let text = {
        let Some(editor) = find_cmd_editor(ctx.desktop_mut()) else {
            return;
        };
        editor.buffer_content()
    };
    // Execute line by line (skip comments and blanks).
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("--") {
            continue;
        }
        exec_text(ctx, state, trimmed);
    }
}

fn exec_text(ctx: &mut CommandContext, state: &mut AppState, text: &str) {
    if text.trim().is_empty() || text.trim().starts_with("--") {
        return;
    }
    let eval_result = state.scripting().eval(text);
    let commands = state.scripting().drain_commands();
    for cmd in commands {
        match execute_command(state, cmd, ctx.desktop_mut()) {
            Ok(msg) => {
                if !msg.is_empty() {
                    status_msg(ctx, &msg);
                }
            }
            Err(e) => status_err(ctx, &e),
        }
    }
    if let Err(e) = eval_result {
        status_err(ctx, &e);
    }
    refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
}

fn find_cmd_editor(desktop: &mut dyn txv_core::prelude::View) -> Option<&mut CommandEditor> {
    let ws = desktop.as_any_mut()?.downcast_mut::<TiledWorkspace>()?;
    let panel = ws.panel_mut(SlotId::Tools as usize)?;
    let count = panel.tab_count();
    let idx = (0..count).find(|&i| {
        panel
            .view_at_mut(i)
            .and_then(|v| v.as_any_mut())
            .is_some_and(|a| a.downcast_ref::<CommandEditor>().is_some())
    })?;
    let view = panel.view_at_mut(idx)?;
    view.as_any_mut()?.downcast_mut::<CommandEditor>()
}

fn handle_exec_command(ctx: &mut CommandContext, state: &mut AppState) {
    let text = ctx
        .data()
        .as_ref()
        .and_then(|d| d.downcast_ref::<String>())
        .cloned()
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }

    // Execute via scripting engine.
    let eval_result = state.scripting().eval(&text);
    let commands = state.scripting().drain_commands();

    for cmd in commands {
        match execute_command(state, cmd, ctx.desktop_mut()) {
            Ok(msg) => {
                if !msg.is_empty() {
                    status_msg(ctx, &msg);
                }
            }
            Err(e) => status_err(ctx, &e),
        }
    }

    if let Err(e) = eval_result {
        status_err(ctx, &e);
    }

    refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
}

fn handle_node_select(ctx: &mut CommandContext, state: &mut AppState) {
    let name = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned();
    let Some(node_name) = name else { return };
    let Some(node) = state.registry.find(&node_name) else {
        return;
    };

    // Polymorphic execution — node decides what to produce.
    let result = node.execute(state.engine());
    let command = node.command().to_string();

    match result {
        Ok(NodeResult::Table(qr)) => {
            insert_table_tab(ctx.desktop_mut(), &node_name, qr, &command);
        }
        Ok(NodeResult::Plot(lines)) => {
            insert_plot_tab(ctx.desktop_mut(), &node_name, &command, lines);
        }
        Ok(NodeResult::Nothing) => {
            status_msg(ctx, &format!("Node '{node_name}' executed (no output)"));
        }
        Err(e) => {
            status_err(ctx, &e);
            if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
                repl.push_error(&e);
            }
        }
    }
}

fn handle_repl_submit(ctx: &mut CommandContext, state: &mut AppState) {
    let input = {
        let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
            return;
        };
        let input = repl.take_input();
        repl.push_command(&input);
        input
    };

    let eval_result = state.scripting().eval(&input);
    let commands = state.scripting().drain_commands();

    let mut output = Vec::new();
    let mut had_error = false;
    for cmd in commands {
        match execute_command(state, cmd, ctx.desktop_mut()) {
            Ok(msg) => {
                if !msg.is_empty() {
                    output.push((false, msg));
                }
            }
            Err(e) => {
                output.push((true, e));
                had_error = true;
            }
        }
    }

    // Refresh lineage tree.
    refresh_lineage_tree(ctx.desktop_mut(), &state.registry);

    let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
        return;
    };
    match eval_result {
        Ok(val) => {
            if !val.is_empty() && output.is_empty() {
                repl.push_output(&val);
            }
            for (is_err, msg) in output {
                if is_err || had_error {
                    repl.push_error(&msg);
                } else {
                    repl.push_output(&msg);
                }
            }
        }
        Err(e) => repl.push_error(&e),
    }
}

fn execute_command(
    state: &mut AppState,
    cmd: ScriptCommand,
    desktop: &mut dyn txv_core::prelude::View,
) -> Result<String, String> {
    match cmd {
        ScriptCommand::Sql { query, var_name } => {
            let result = state.engine().query(&query)?;
            let msg = format!("{} rows, {} cols", result.row_count, result.columns.len());

            let upper = query.trim().to_uppercase();
            if upper.starts_with("CREATE") {
                let table_name = sql_analysis::extract_created_table(&query).unwrap_or_else(|| "result".to_string());
                log::info!("CREATE detected, table_name='{table_name}'");
                state
                    .registry
                    .add_table(&table_name, &format!("sql {{{query}}}"), &query, None);
            } else if let Some(tab_name) = var_name {
                // Named query: register node + create view.
                let parent = sql_analysis::detect_parent_table(&query);
                let full_cmd = format!("sql -name {tab_name} {{{query}}}");
                state.registry.add_query(
                    &tab_name,
                    &full_cmd,
                    &query,
                    parent.as_deref(),
                    Some(result.row_count as u64),
                );
                let _ = state
                    .engine()
                    .query(&format!("CREATE OR REPLACE VIEW \"{tab_name}\" AS {query}"));
                insert_table_tab(desktop, &tab_name, result, &query);
            } else {
                // Unnamed: just show results, no node, no view.
                insert_table_tab(desktop, "result", result, &query);
            }
            Ok(msg)
        }
        ScriptCommand::Into { table, source, .. } => match source {
            crate::scripting::ImportSource::File(path) => {
                let p = std::path::Path::new(&path);
                let result = state.engine().import_csv(p, &table)?;
                let count: u64 = result
                    .rows
                    .first()
                    .and_then(|r| r.first())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let full_cmd = format!("into {table} -file {path}");
                let create_sql =
                    format!("CREATE OR REPLACE TABLE \"{table}\" AS SELECT * FROM read_csv_auto('{path}')");
                state.registry.add_table(&table, &full_cmd, &create_sql, Some(count));
                let preview = state.engine().query(&format!("SELECT * FROM \"{table}\" LIMIT 100"));
                if let Ok(data) = preview {
                    insert_table_tab(desktop, &table, data, &full_cmd);
                }
                Ok(format!("Imported {count} rows → {table}"))
            }
            crate::scripting::ImportSource::Exec(_) => Err("exec import not yet implemented".into()),
        },
        ScriptCommand::Plot {
            plot_type,
            data_ref,
            options,
        } => {
            let cmd = format!("plot {plot_type} {data_ref} {}", options.join(" "));
            let columns = options;
            let tab_name = format!("plot:{data_ref}");
            state
                .registry
                .add_plot(&tab_name, &cmd, &plot_type, &data_ref, &columns);

            // Execute the newly created node.
            let Some(node) = state.registry.find(&tab_name) else {
                return Err("failed to create plot node".into());
            };
            match node.execute(state.engine()) {
                Ok(NodeResult::Plot(lines)) => {
                    insert_plot_tab(desktop, &tab_name, &cmd, lines);
                    Ok(tab_name)
                }
                Ok(_) => Ok(tab_name),
                Err(e) => Err(e),
            }
        }
        ScriptCommand::Derive { name, sql } => {
            let parent = sql_analysis::detect_parent_table(&sql);
            let full_cmd = format!("derive {name} {{{sql}}}");
            state
                .registry
                .add_query(&name, &full_cmd, &sql, parent.as_deref(), None);
            Ok(format!("Created node: {name}"))
        }
        ScriptCommand::Freeze => Ok("Freeze: not yet implemented".into()),
        ScriptCommand::Run => Ok("Run: not yet implemented".into()),
        ScriptCommand::Export { .. } => Ok("Export: not yet implemented".into()),
        ScriptCommand::Budget { .. } => Ok("Budget: not yet implemented".into()),
    }
}

// ─── UI helpers ────────────────────────────────────────────────────────

fn find_repl_mut(desktop: &mut dyn txv_core::prelude::View) -> Option<&mut ReplView> {
    let ws = desktop.as_any_mut()?.downcast_mut::<TiledWorkspace>()?;
    let panel = ws.panel_mut(SlotId::Tools as usize)?;
    let count = panel.tab_count();
    let idx = (0..count).find(|&i| {
        panel
            .view_at_mut(i)
            .and_then(|v| v.as_any_mut())
            .is_some_and(|a| a.downcast_ref::<ReplView>().is_some())
    })?;
    let view = panel.view_at_mut(idx)?;
    view.as_any_mut()?.downcast_mut::<ReplView>()
}

fn refresh_lineage_tree(desktop: &mut dyn txv_core::prelude::View, registry: &registry::Registry) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let Some(panel) = ws.panel_mut(SlotId::Left as usize) else {
        return;
    };
    let count = panel.tab_count();
    let idx = (0..count).find(|&i| {
        panel
            .view_at_mut(i)
            .and_then(|v| v.as_any_mut())
            .is_some_and(|a| a.downcast_ref::<LineageTreeView>().is_some())
    });
    if let Some(i) = idx {
        if let Some(tree) = panel
            .view_at_mut(i)
            .and_then(|v| v.as_any_mut())
            .and_then(|a| a.downcast_mut::<LineageTreeView>())
        {
            tree.inner.data_mut().update_from_registry(registry);
        }
    }
}

fn insert_table_tab(
    desktop: &mut dyn txv_core::prelude::View,
    name: &str,
    result: crate::engine::QueryResult,
    command: &str,
) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Center as usize;
    #[allow(deprecated)]
    if let Some(panel) = ws.panel_mut(slot) {
        panel.close_tab_by_title(name);
    }
    let mut view = TableView::new(name, result);
    if !command.is_empty() {
        view.set_command(command);
    }
    ws.insert_tab(slot, name, Box::new(view));
}

fn insert_plot_tab(desktop: &mut dyn txv_core::prelude::View, name: &str, command: &str, lines: Vec<String>) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Center as usize;
    #[allow(deprecated)]
    if let Some(panel) = ws.panel_mut(slot) {
        panel.close_tab_by_title(name);
    }
    ws.insert_tab(slot, name, Box::new(PlotView::new(name, command, lines)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;

    fn test_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        AppState::new(dir.into_path())
    }

    #[test]
    fn test_sql_creates_table_node() {
        let mut state = test_state();
        let mut ws = crate::workspace::build_workspace(std::path::Path::new("/tmp"));
        state.scripting().eval("sql {CREATE TABLE t AS SELECT 1 as x}").unwrap();
        let cmds = state.scripting().drain_commands();
        let msg = execute_command(&mut state, cmds.into_iter().next().unwrap(), &mut ws);
        assert!(msg.is_ok());
        assert!(state.registry.find("t").is_some());
        assert_eq!(state.registry.find("t").unwrap().icon(), "[T]");
    }

    #[test]
    fn test_sql_creates_query_node() {
        let mut state = test_state();
        let mut ws = crate::workspace::build_workspace(std::path::Path::new("/tmp"));
        state.engine().query("CREATE TABLE t AS SELECT 1 as x").unwrap();
        state.scripting().eval("sql -name q {SELECT * FROM t}").unwrap();
        let cmds = state.scripting().drain_commands();
        execute_command(&mut state, cmds.into_iter().next().unwrap(), &mut ws).unwrap();
        assert!(state.registry.find("q").is_some());
        assert_eq!(state.registry.find("q").unwrap().icon(), "[Q]");
    }

    #[test]
    fn test_node_execute_polymorphic() {
        let mut state = test_state();
        state.engine().query("CREATE TABLE t AS SELECT 1 as x, 2 as y").unwrap();
        state.registry.add_query("q", "cmd", "SELECT * FROM t", None, None);
        let node = state.registry.find("q").unwrap();
        let result = node.execute(state.engine());
        match result.unwrap() {
            NodeResult::Table(qr) => assert_eq!(qr.row_count, 1),
            _ => panic!("expected Table"),
        }
    }
}
