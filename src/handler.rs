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
use crate::status::{CM_APP_QUIT, CM_CONFIRM_ACTIVATE, CM_CONFIRM_RESPONSE, CM_EXECUTE_COMMAND, CM_SHOW_HELP};
use crate::views::cmd_editor::{CommandEditor, CM_EDITOR_COMPLETE, CM_EXEC_BUFFER, CM_EXEC_LINE};
use crate::views::help::HelpView;
use crate::views::lineage_tree::{LineageTreeView, CM_NODE_CLONE, CM_NODE_DELETE, CM_NODE_EDIT, CM_NODE_SELECT, CM_NODE_SELECT_FOCUS};
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
    // Poll background jobs on every command (cheap no-op if empty).
    poll_jobs(ctx, state);

    match ctx.command() {
        CM_REPL_SUBMIT => handle_repl_submit(ctx, state),
        CM_REPL_TAB => handle_repl_tab(ctx, state),
        CM_EXEC_LINE => handle_exec_line(ctx, state),
        CM_EXEC_BUFFER => handle_exec_buffer(ctx, state),
        CM_EDITOR_COMPLETE => handle_editor_complete(ctx, state),
        CM_NODE_SELECT => handle_node_select(ctx, state),
        CM_NODE_SELECT_FOCUS => {
            handle_node_select(ctx, state);
            ctx.sink().push_command(txv_widgets::tiled_workspace::commands::CM_TW_FOCUS_PANEL, Some(Box::new(SlotId::Center as usize)));
        }
        CM_NODE_EDIT => handle_node_edit(ctx, state),
        CM_NODE_DELETE => handle_node_delete(ctx, state),
        CM_NODE_CLONE => handle_node_clone(ctx, state),
        CM_CONFIRM_RESPONSE => handle_confirm_response(ctx, state),
        CM_SIDEKICK_RESULT => {
            // User picked from dropdown — apply completion (String payload).
            if let Some(text) = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()) {
                let text = text.clone();
                let repl_active = find_repl_mut(ctx.desktop_mut())
                    .map(|r| r.sidekick_visible)
                    .unwrap_or(false);
                if repl_active {
                    if let Some(repl) = find_repl_mut(ctx.desktop_mut()) {
                        repl.apply_completion(&text);
                        repl.sidekick_visible = false;
                    }
                } else if let Some(editor) = find_cmd_editor(ctx.desktop_mut()) {
                    // Replace word prefix at cursor with the selected completion
                    let e = editor.inner.editor_mut();
                    let line = e.cursor_line();
                    let col = e.cursor_col();
                    let line_text = e.buf().line(line).unwrap_or_default();
                    let chars: Vec<char> = line_text.chars().collect();
                    let prefix_len = chars[..col].iter().rev()
                        .take_while(|c| c.is_alphanumeric() || **c == '_')
                        .count();
                    let word_start = col - prefix_len;
                    let start_offset = e.buf().line_col_to_offset(line, word_start).unwrap_or(0);
                    let end_offset = e.buf().line_col_to_offset(line, col).unwrap_or(start_offset);
                    if end_offset > start_offset {
                        e.buf().delete(start_offset, end_offset);
                    }
                    e.buf().insert(start_offset, &text);
                    e.set_cursor_col(word_start + text.len());
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
        CM_EXECUTE_COMMAND => handle_mx_command(ctx, state),
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

/// Public accessor for session save/restore.
pub fn find_cmd_editor_pub(desktop: &mut dyn txv_core::prelude::View) -> Option<&mut CommandEditor> {
    find_cmd_editor(desktop)
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

fn handle_node_edit(ctx: &mut CommandContext, state: &mut AppState) {
    let name = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned();
    let Some(node_name) = name else { return };
    let Some(node) = state.registry.find(&node_name) else {
        return;
    };
    let command = node.command().to_string();

    if let Some(editor) = find_cmd_editor(ctx.desktop_mut()) {
        editor.set_content(&command);
    }
}

fn handle_node_delete(ctx: &mut CommandContext, state: &mut AppState) {
    let name = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned();
    let Some(node_name) = name else { return };
    if state.registry.find(&node_name).is_none() {
        return;
    }
    state.pending_delete = Some(node_name.clone());
    let prompt = format!("Delete '{node_name}' and children?");
    ctx.sink().push_command(CM_CONFIRM_ACTIVATE, Some(Box::new(prompt)));
}

fn handle_confirm_response(ctx: &mut CommandContext, state: &mut AppState) {
    let ch = ctx.data().as_ref().and_then(|d| d.downcast_ref::<char>()).copied();
    let Some(node_name) = state.pending_delete.take() else { return };
    if ch == Some('y') {
        let removed = state.registry.remove_subtree(&node_name);
        refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
        status_msg(ctx, &format!("Deleted {} node(s): {}", removed.len(), removed.join(", ")));
    } else {
        status_msg(ctx, "Delete cancelled");
    }
}

fn handle_node_clone(ctx: &mut CommandContext, state: &mut AppState) {
    let name = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned();
    let Some(node_name) = name else { return };
    if state.registry.find(&node_name).is_none() {
        return;
    }
    let cloned = state.registry.clone_subtree(&node_name, "_copy");
    refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
    status_msg(ctx, &format!("Cloned {} node(s): {}", cloned.len(), cloned.join(", ")));
}

fn handle_mx_command(ctx: &mut CommandContext, state: &mut AppState) {
    let text = match ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()) {
        Some(t) => t.clone(),
        None => return,
    };
    let trimmed = text.trim().strip_prefix(':').unwrap_or(text.trim());
    let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
    let cmd = parts.first().copied().unwrap_or("");
    let arg = parts.get(1).copied().unwrap_or("");

    match cmd {
        "shell" => {
            open_shell_tab(ctx.desktop_mut(), state);
            status_msg(ctx, "Shell opened");
        }
        "kiro" => {
            handle_mx_kiro(ctx, state, arg);
        }
        "help" => {
            if let Some(ws) = ctx.desktop_mut().as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) {
                ws.insert_tab(SlotId::Center as usize, "Help", Box::new(HelpView::new()));
            }
        }
        "quit" | "q" => {
            ctx.sink().push_command(txv_core::commands::CM_QUIT, None);
        }
        _ => {
            // Fall through to Tcl eval
            match state.scripting().eval(trimmed) {
                Ok(result) => {
                    let commands = state.scripting().drain_commands();
                    let had_commands = !commands.is_empty();
                    for c in commands {
                        match execute_command(state, c, ctx.desktop_mut()) {
                            Ok(msg) => {
                                refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
                                if !msg.is_empty() {
                                    status_msg(ctx, &msg);
                                }
                            }
                            Err(e) => status_err(ctx, &e),
                        }
                    }
                    if !had_commands && !result.is_empty() {
                        status_msg(ctx, &result);
                    }
                }
                Err(e) => status_err(ctx, &format!("Unknown: {e}")),
            }
        }
    }
}

fn handle_mx_kiro(ctx: &mut CommandContext, state: &mut AppState, arg: &str) {
    let args: Vec<&str> = arg.split_whitespace().collect();
    let agent = extract_agent_arg(&args);
    let extra: Vec<&str> = args.iter().copied().filter(|a| !a.starts_with("--agent")).collect();
    open_kiro_tab(ctx.desktop_mut(), state, agent, &extra);
    status_msg(ctx, "Kiro launched");
}

fn extract_agent_arg<'a>(args: &'a [&str]) -> Option<&'a str> {
    for (i, arg) in args.iter().enumerate() {
        if let Some(name) = arg.strip_prefix("--agent=") {
            return Some(name);
        }
        if *arg == "--agent" {
            return args.get(i + 1).copied();
        }
    }
    None
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
                let parents = sql_analysis::extract_table_refs(&query);
                let full_cmd = format!("sql -name {tab_name} {{{query}}}");
                state
                    .registry
                    .add_query_multi(&tab_name, &full_cmd, &query, &parents, Some(result.row_count as u64));
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
            crate::scripting::ImportSource::Exec(cmd) => {
                // Async: create node as Running, spawn background job
                let full_cmd_str = format!("into {table} --shell {{{cmd}}}");
                state.registry.add_table(&table, &full_cmd_str, "", None);
                state.registry.set_node_status(&table, crate::node_state::NodeStatus::Running);
                let (handle, _cancel) = crate::engine::Engine::spawn_shell_import(&state.root_dir, &cmd, &table);
                state.jobs.register(handle);
                Ok(format!("Started: {table} (shell import)"))
            }
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
            let parents = sql_analysis::extract_table_refs(&sql);
            let full_cmd = format!("derive {name} {{{sql}}}");
            state.registry.add_query_multi(&name, &full_cmd, &sql, &parents, None);
            Ok(format!("Created node: {name}"))
        }
        ScriptCommand::Freeze => Ok("Freeze: not yet implemented".into()),
        ScriptCommand::Run => Ok("Run: not yet implemented".into()),
        ScriptCommand::Export { .. } => Ok("Export: not yet implemented".into()),
        ScriptCommand::Budget { .. } => Ok("Budget: not yet implemented".into()),
        ScriptCommand::Shell => {
            open_shell_tab(desktop, state);
            Ok("Shell opened".into())
        }
        ScriptCommand::Kiro { agent } => {
            open_kiro_tab(desktop, state, agent.as_deref(), &[]);
            Ok("Kiro launched".into())
        }
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



fn handle_editor_complete(ctx: &mut CommandContext, state: &mut AppState) {
    let prefix = ctx.data().as_ref().and_then(|d| d.downcast_ref::<String>()).cloned().unwrap_or_default();

    // Gather completions: commands + table names
    let keywords = ["sql", "into", "plot", "derive", "export", "freeze", "run", "shell", "kiro"];
    let mut items: Vec<String> = keywords.iter()
        .filter(|k| k.starts_with(prefix.as_str()))
        .map(|k| k.to_string())
        .collect();

    for node in state.registry.nodes() {
        let name = &node.name;
        if name.starts_with(prefix.as_str()) && !items.contains(name) {
            items.push(name.clone());
        }
    }

    if items.is_empty() {
        return;
    }

    // Show dropdown via sidekick
    use txv_widgets::sidekick::SidekickRequest;
    use txv_widgets::DropdownMenu;

    let source = SimpleSource(items);
    let menu = DropdownMenu::new(source);
    let rect = txv_core::prelude::Rect::new(0, 0, 30, 8);
    let view_id = find_cmd_editor(ctx.desktop_mut())
        .map(|e| txv_core::prelude::View::view_id(e))
        .unwrap_or(0);
    let request = SidekickRequest::new(Box::new(menu), rect, view_id);
    ctx.sink().push_command(txv_widgets::sidekick::CM_SIDEKICK_SHOW, Some(Box::new(request)));
}

/// Simple DropdownSource from a list of strings.
struct SimpleSource(Vec<String>);

impl txv_widgets::DropdownSource for SimpleSource {
    fn len(&self) -> usize { self.0.len() }
    fn label(&self, idx: usize) -> &str { &self.0[idx] }
}

fn poll_jobs(ctx: &mut CommandContext, state: &mut AppState) {
    let completed = state.jobs.poll();
    if completed.is_empty() {
        return;
    }
    for (node_id, result) in &completed {
        match result {
            Ok(msg) => {
                state.registry.set_node_status(node_id, crate::node_state::NodeStatus::UpToDate);
                status_msg(ctx, &format!("{node_id}: {msg}"));
            }
            Err(e) => {
                state.registry.set_node_status(node_id, crate::node_state::NodeStatus::Error(e.clone()));
                status_err(ctx, &format!("{node_id}: {e}"));
            }
        }
    }
    refresh_lineage_tree(ctx.desktop_mut(), &state.registry);
}

fn open_shell_tab(desktop: &mut dyn txv_core::prelude::View, state: &AppState) {
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Tools as usize;
    let term = new_shell_terminal(&state.root_dir);
    ws.insert_tab(slot, "Shell", term);
    if let Some(panel) = ws.panel_mut(slot) {
        let count = panel.tab_count();
        panel.set_active(count.saturating_sub(1));
    }
}

fn open_kiro_tab(desktop: &mut dyn txv_core::prelude::View, state: &AppState, agent: Option<&str>, extra_args: &[&str]) {
    let agent_name = agent.unwrap_or("tplot");
    let patched_agent = match crate::mcp::agent_patch::ensure_agent_patched(&state.root_dir, agent_name) {
        Ok(name) => name,
        Err(e) => {
            log::error!("kiro agent patch failed: {e}");
            return;
        }
    };

    let argv = build_kiro_argv(&patched_agent, extra_args);
    let Some(ws) = desktop.as_any_mut().and_then(|a| a.downcast_mut::<TiledWorkspace>()) else {
        return;
    };
    let slot = SlotId::Tools as usize;
    let term = new_kiro_terminal(&argv, &state.root_dir);
    ws.insert_tab(slot, "Kiro", term);
    if let Some(panel) = ws.panel_mut(slot) {
        let count = panel.tab_count();
        panel.set_active(count.saturating_sub(1));
    }
}

fn build_kiro_argv(agent: &str, extra_args: &[&str]) -> Vec<String> {
    let mut argv = vec!["kiro-cli".to_string(), "chat".to_string(), format!("--agent={agent}")];
    argv.extend(extra_args.iter().filter(|a| !a.starts_with("--agent")).map(|s| s.to_string()));
    argv
}

fn new_shell_terminal(_cwd: &std::path::Path) -> Box<dyn txv_core::prelude::View> {
    if std::env::var("TPLOT_TEST").is_ok() {
        return Box::new(crate::views::placeholder::PlaceholderView::new("Shell"));
    }
    match txv_widgets::PtyTerminal::spawn_shell(80, 24) {
        Ok(term) => Box::new(term),
        Err(e) => {
            log::error!("Shell spawn failed: {e}");
            Box::new(crate::views::placeholder::PlaceholderView::new("Shell (failed)"))
        }
    }
}

fn new_kiro_terminal(argv: &[String], cwd: &std::path::Path) -> Box<dyn txv_core::prelude::View> {
    if std::env::var("TPLOT_TEST").is_ok() {
        return Box::new(crate::views::placeholder::PlaceholderView::new("Kiro"));
    }
    let (program, args) = match argv.split_first() {
        Some((p, a)) => (p.as_str(), a),
        None => return Box::new(crate::views::placeholder::PlaceholderView::new("Kiro (failed)")),
    };
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let socket_val = std::env::var("TPLOT_MCP_SOCKET").unwrap_or_default();
    let envs: Vec<(&str, &str)> = if socket_val.is_empty() {
        vec![]
    } else {
        vec![("TPLOT_MCP_SOCKET", &socket_val)]
    };
    match txv_widgets::PtyTerminal::spawn_command_with_env(program, &arg_refs, cwd, 80, 24, &envs) {
        Ok(term) => Box::new(term),
        Err(e) => {
            log::error!("Kiro spawn failed: {e}");
            Box::new(crate::views::placeholder::PlaceholderView::new("Kiro (failed)"))
        }
    }
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
