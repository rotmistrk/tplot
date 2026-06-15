//! Command handler — dispatches commands from the TUI event loop.

use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::app::AppState;
use crate::registry::NodeRegistry;
use crate::scripting::ScriptCommand;
use crate::slots::SlotId;
use crate::status::{CM_APP_QUIT, CM_SHOW_HELP};
use crate::views::help::HelpView;
use crate::views::lineage_tree::{LineageTreeView, CM_NODE_SELECT};
use crate::views::repl::{ReplView, CM_REPL_SUBMIT};
use crate::views::table::TableView;

pub(crate) fn handle_command(ctx: &mut CommandContext, state: &mut AppState) {
    let cmd = ctx.command();
    match cmd {
        CM_REPL_SUBMIT => handle_repl_submit(ctx, state),
        CM_NODE_SELECT => handle_node_select(ctx, state),
        CM_APP_QUIT => {
            ctx.sink().push_command(txv_core::commands::CM_QUIT, None);
        }
        CM_SHOW_HELP => {
            let ws = ctx
                .desktop_mut()
                .as_any_mut()
                .and_then(|a| a.downcast_mut::<TiledWorkspace>());
            if let Some(ws) = ws {
                let slot = SlotId::Center as usize;
                ws.insert_tab(slot, "Help", Box::new(HelpView::new()));
            }
        }
        _ => {}
    }
}

fn handle_node_select(ctx: &mut CommandContext, state: &mut AppState) {
    let name = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned();
    let Some(node_name) = name else { return };

    let node = state.registry.nodes().iter().find(|n| n.name == node_name).cloned();
    let Some(node) = node else { return };

    // Show command in REPL.
    if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
        repl.push_output(&format!("[{node_name}] {}", node.command));
    }

    // Re-run query and show result.
    let cmd = &node.command;
    if let Some(query) = cmd.strip_prefix("sql -name ") {
        if let Some(start) = query.find('{') {
            let sql = &query[start + 1..query.len() - 1];
            if let Ok(result) = state.engine().query(sql) {
                insert_table_tab(ctx.desktop_mut(), &node_name, result);
            }
        }
    } else if let Some(query) = cmd.strip_prefix("sql {") {
        let sql = &query[..query.len() - 1];
        let _ = state.engine().query(sql);
    }
}

fn handle_repl_submit(ctx: &mut CommandContext, state: &mut AppState) {
    // Get input from REPL.
    let input = {
        let Some(repl) = find_repl_mut(ctx.desktop_mut()) else {
            return;
        };
        let input = repl.take_input();
        repl.push_command(&input);
        input
    };

    // Execute via scripting engine.
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

    // Push results back to REPL.
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

fn execute_command(
    state: &mut AppState,
    cmd: ScriptCommand,
    desktop: &mut dyn txv_core::prelude::View,
) -> Result<String, String> {
    match cmd {
        ScriptCommand::Sql { query, var_name } => {
            let result = state.engine().query(&query)?;
            let msg = format!("{} rows, {} cols", result.row_count, result.columns.len());
            let tab_name = var_name.unwrap_or_else(|| "result".to_string());

            // Detect CREATE TABLE vs SELECT.
            let upper = query.trim().to_uppercase();
            if upper.starts_with("CREATE") {
                let table_name = extract_create_table_name(&query).unwrap_or_else(|| tab_name.clone());
                let full_cmd = format!("sql {{{query}}}");
                state.registry.add_table(&table_name, &full_cmd, None);
            } else {
                let parent = crate::registry::detect_parent_table(&query);
                let full_cmd = format!("sql -name {tab_name} {{{query}}}");
                state
                    .registry
                    .add_query(&tab_name, &full_cmd, parent.as_deref(), Some(result.row_count as u64));
                insert_table_tab(desktop, &tab_name, result);
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

                // Register as materialized table.
                let full_cmd = format!("into {table} -file {path}");
                state.registry.add_table(&table, &full_cmd, Some(count));

                // Show preview.
                let preview = state.engine().query(&format!("SELECT * FROM \"{table}\" LIMIT 100"));
                if let Ok(data) = preview {
                    insert_table_tab(desktop, &table, data);
                }
                Ok(format!("Imported {count} rows → {table}"))
            }
            crate::scripting::ImportSource::Exec(_) => Err("exec import not yet implemented".into()),
        },
        ScriptCommand::Derive { name, sql } => {
            let parent = crate::registry::detect_parent_table(&sql);
            let full_cmd = format!("derive {name} {{{sql}}}");
            state.registry.add_query(&name, &full_cmd, parent.as_deref(), None);
            Ok(format!("Created node: {name}"))
        }
        ScriptCommand::Freeze => Ok("Node frozen".into()),
        ScriptCommand::Run => Ok("Run: not yet implemented".into()),
        ScriptCommand::Plot { .. } => Ok("Plot: not yet implemented".into()),
        ScriptCommand::Export { .. } => Ok("Export: not yet implemented".into()),
        ScriptCommand::Budget { .. } => Ok("Budget: not yet implemented".into()),
    }
}

/// Refresh the lineage tree view with current registry state.
fn refresh_lineage_tree(desktop: &mut dyn txv_core::prelude::View, registry: &NodeRegistry) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Left as usize;
    let Some(panel) = ws.panel_mut(slot) else { return };
    let count = panel.tab_count();
    let idx = (0..count).find(|&i| {
        panel
            .view_at_mut(i)
            .and_then(|v| v.as_any_mut())
            .is_some_and(|a| a.downcast_ref::<LineageTreeView>().is_some())
    });
    if let Some(i) = idx {
        let view = panel.view_at_mut(i).unwrap();
        if let Some(tree) = view.as_any_mut().and_then(|a| a.downcast_mut::<LineageTreeView>()) {
            let nodes = registry.nodes().to_vec();
            tree.inner.data_mut().update(nodes);
        }
    }
}

/// Extract table name from "CREATE TABLE <name> AS ..." or "CREATE OR REPLACE TABLE <name> ...".
fn extract_create_table_name(sql: &str) -> Option<String> {
    let upper = sql.to_uppercase();
    let table_pos = upper.find("TABLE ")?;
    let after = &sql[table_pos + 6..];
    let name = after
        .trim()
        .split(|c: char| c.is_whitespace() || c == '(' || c == ';')
        .next()?
        .trim_matches('"');
    if name.is_empty() || name.to_uppercase() == "AS" || name.to_uppercase() == "IF" {
        return None;
    }
    Some(name.to_string())
}

fn insert_table_tab(desktop: &mut dyn txv_core::prelude::View, name: &str, result: crate::engine::QueryResult) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Center as usize;
    // Remove existing tab with same name.
    #[allow(deprecated)]
    if let Some(panel) = ws.panel_mut(slot) {
        panel.close_tab_by_title(name);
    }
    let view = TableView::new(name, result);
    ws.insert_tab(slot, name, Box::new(view));
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
    fn test_sql_execution() {
        let mut state = test_state();
        state.engine().query("CREATE TABLE t (x INT)").unwrap();
        state.engine().query("INSERT INTO t VALUES (1),(2),(3)").unwrap();

        state.scripting().eval("sql {SELECT count(*) FROM t}").unwrap();
        let cmds = state.scripting().drain_commands();
        // Use a dummy desktop (won't insert tabs in test).
        let mut ws = crate::workspace::build_workspace(std::path::Path::new("/tmp"));
        let msg = execute_command(&mut state, cmds.into_iter().next().unwrap(), &mut ws);
        assert!(msg.unwrap().contains("1 rows"));
    }

    #[test]
    fn test_into_file() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("d.csv");
        std::fs::write(&csv, "a,b\n1,x\n2,y\n").unwrap();

        let mut state = AppState::new(dir.into_path());
        let path = csv.to_string_lossy();
        state.scripting().eval(&format!("into tbl -file {path}")).unwrap();
        let cmds = state.scripting().drain_commands();
        let mut ws = crate::workspace::build_workspace(std::path::Path::new("/tmp"));
        let msg = execute_command(&mut state, cmds.into_iter().next().unwrap(), &mut ws);
        assert!(msg.unwrap().contains("2 rows"));
    }
}
