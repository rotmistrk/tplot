//! Command handler — dispatches commands from the TUI event loop.

use txv_core::program::CommandContext;
use txv_widgets::tiled_workspace::TiledWorkspace;

use crate::app::AppState;
use crate::scripting::ScriptCommand;
use crate::slots::SlotId;
use crate::views::repl::{ReplView, CM_REPL_SUBMIT};

pub(crate) fn handle_command(ctx: &mut CommandContext, state: &mut AppState) {
    let cmd = ctx.command();
    if cmd == CM_REPL_SUBMIT {
        handle_repl_submit(ctx, state);
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
        match execute_command(state, cmd) {
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

fn execute_command(state: &mut AppState, cmd: ScriptCommand) -> Result<String, String> {
    match cmd {
        ScriptCommand::Sql { query, .. } => {
            let result = state.engine().query(&query)?;
            Ok(format!("{} rows, {} cols", result.row_count, result.columns.len()))
        }
        ScriptCommand::Into { table, source, .. } => match source {
            crate::scripting::ImportSource::File(path) => {
                let p = std::path::Path::new(&path);
                let result = state.engine().import_csv(p, &table)?;
                let count = result.rows.first().and_then(|r| r.first()).cloned().unwrap_or_default();
                Ok(format!("Imported {count} rows → {table}"))
            }
            crate::scripting::ImportSource::Exec(_) => Err("exec import not yet implemented".into()),
        },
        ScriptCommand::Derive { name, sql } => Ok(format!("Created node: {name} ({sql})")),
        ScriptCommand::Freeze => Ok("Node frozen".into()),
        ScriptCommand::Run => Ok("Run: not yet implemented".into()),
        ScriptCommand::Plot { .. } => Ok("Plot: not yet implemented".into()),
        ScriptCommand::Export { .. } => Ok("Export: not yet implemented".into()),
        ScriptCommand::Budget { .. } => Ok("Budget: not yet implemented".into()),
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
    fn test_sql_execution() {
        let mut state = test_state();
        state.engine().query("CREATE TABLE t (x INT)").unwrap();
        state.engine().query("INSERT INTO t VALUES (1),(2),(3)").unwrap();

        state.scripting().eval("sql {SELECT count(*) FROM t}").unwrap();
        let cmds = state.scripting().drain_commands();
        let msg = execute_command(&mut state, cmds.into_iter().next().unwrap());
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
        let msg = execute_command(&mut state, cmds.into_iter().next().unwrap());
        assert!(msg.unwrap().contains("2 rows"));
    }
}
