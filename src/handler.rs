//! Command handler — dispatches commands from the TUI event loop.

use txv_core::program::CommandContext;

use crate::app::AppState;
use crate::scripting::ScriptCommand;

pub(crate) fn handle_command(_ctx: &mut CommandContext, _state: &mut AppState) {
    // Will dispatch TUI commands as features are added.
}

/// Execute a tplot script line (from REPL or node script).
/// Returns Ok(output) or Err(error message).
#[allow(dead_code)]
pub(crate) fn execute_script_line(state: &mut AppState, line: &str) -> Result<String, String> {
    let result = state.scripting().eval(line)?;
    let commands = state.scripting().drain_commands();
    for cmd in commands {
        execute_command(state, cmd)?;
    }
    Ok(result)
}

fn execute_command(state: &mut AppState, cmd: ScriptCommand) -> Result<(), String> {
    match cmd {
        ScriptCommand::Sql { query, .. } => {
            let result = state.engine().query(&query)?;
            log::info!("SQL: {} rows, {} cols", result.row_count, result.columns.len());
            Ok(())
        }
        ScriptCommand::Into { table, source, .. } => {
            match source {
                crate::scripting::ImportSource::File(path) => {
                    let path = std::path::Path::new(&path);
                    state.engine().import_csv(path, &table)?;
                }
                crate::scripting::ImportSource::Exec(_cmd) => {
                    log::warn!("exec import not yet implemented");
                }
            }
            Ok(())
        }
        ScriptCommand::Derive { name, sql } => {
            log::info!("derive {name}: {sql}");
            // TODO: create child node with this SQL as script
            Ok(())
        }
        ScriptCommand::Freeze => {
            log::info!("freeze current node");
            Ok(())
        }
        ScriptCommand::Run => {
            log::info!("run current node");
            Ok(())
        }
        ScriptCommand::Plot { .. } => {
            log::info!("plot (not yet implemented)");
            Ok(())
        }
        ScriptCommand::Export { .. } => {
            log::info!("export (not yet implemented)");
            Ok(())
        }
        ScriptCommand::Budget { .. } => {
            log::info!("budget (not yet implemented)");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        AppState::new(dir.into_path())
    }

    #[test]
    fn test_sql_execution_pipeline() {
        let mut state = test_state();
        // Create a table via DuckDB directly
        state.engine().query("CREATE TABLE t (x INT, y TEXT)").unwrap();
        state.engine().query("INSERT INTO t VALUES (1, 'a'), (2, 'b')").unwrap();

        // Execute SQL via Tcl scripting
        let result = execute_script_line(&mut state, "sql {SELECT count(*) as n FROM t}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_into_file_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("data.csv");
        std::fs::write(&csv, "a,b\n1,hello\n2,world\n").unwrap();

        let mut state = AppState::new(dir.into_path());
        let csv_str = csv.to_string_lossy().to_string();
        let cmd = format!("into test_table -file {csv_str}");
        let result = execute_script_line(&mut state, &cmd);
        assert!(result.is_ok(), "got: {result:?}");

        // Verify data is in DuckDB
        let qr = state.engine().query("SELECT count(*) FROM test_table").unwrap();
        assert_eq!(qr.rows[0][0], "2");
    }
}
