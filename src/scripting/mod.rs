//! Tcl scripting engine — embeds rusticle interpreter with tplot bridge commands.

mod bridge;

use std::sync::{Arc, Mutex};

use rusticle::interpreter::Interpreter;

/// Commands produced by Tcl scripts for execution by the app.
#[derive(Debug, Clone)]
pub(crate) enum ScriptCommand {
    /// Execute SQL and store result in a variable.
    Sql { query: String, var_name: Option<String> },
    /// Import data from source into a table.
    Into {
        table: String,
        source: ImportSource,
        format: ImportFormat,
    },
    /// Render a plot.
    Plot {
        plot_type: String,
        data_ref: String,
        options: Vec<String>,
    },
    /// Create a child node.
    Derive { name: String, sql: String },
    /// Export data to file.
    Export {
        data_ref: String,
        format: ExportFormat,
        path: String,
    },
    /// Set resource budget.
    Budget {
        cpu: Option<u32>,
        ram_mb: Option<u64>,
        disk_mb: Option<u64>,
    },
    /// Freeze current node.
    Freeze,
    /// Re-run current node's script.
    Run,
}

/// Source for the `into` command.
#[derive(Debug, Clone)]
pub(crate) enum ImportSource {
    Exec(String),
    File(String),
}

/// Format hint for parsing imported data.
#[derive(Debug, Clone)]
pub(crate) enum ImportFormat {
    Auto,
    Csv,
    Tsv,
    Json,
    Sep(String),
    Regex { pattern: String, cols: Vec<String> },
}

/// Export format.
#[derive(Debug, Clone)]
pub(crate) enum ExportFormat {
    Csv,
    JsonL,
    Parquet,
    Png,
    Svg,
}

/// The scripting engine: interpreter + command queue.
pub(crate) struct ScriptEngine {
    interp: Interpreter,
    commands: Arc<Mutex<Vec<ScriptCommand>>>,
}

impl ScriptEngine {
    pub(crate) fn new() -> Self {
        let commands: Arc<Mutex<Vec<ScriptCommand>>> = Arc::new(Mutex::new(Vec::new()));
        let mut interp = Interpreter::new();
        bridge::register(&mut interp, commands.clone());
        Self { interp, commands }
    }

    /// Execute a Tcl script, return any error message.
    pub(crate) fn eval(&mut self, script: &str) -> Result<String, String> {
        self.interp
            .eval(script)
            .map(|v| v.to_string())
            .map_err(|e| e.to_string())
    }

    /// Drain pending commands produced by the last eval.
    pub(crate) fn drain_commands(&self) -> Vec<ScriptCommand> {
        let mut guard = self.commands.lock().unwrap_or_else(|p| p.into_inner());
        std::mem::take(&mut *guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_command() {
        let mut engine = ScriptEngine::new();
        let result = engine.eval("sql {SELECT 1}");
        assert!(result.is_ok());

        let cmds = engine.drain_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            ScriptCommand::Sql { query, .. } => assert_eq!(query, "SELECT 1"),
            _ => panic!("expected Sql command"),
        }
    }

    #[test]
    fn test_into_command() {
        let mut engine = ScriptEngine::new();
        engine.eval("into flows data.csv -csv").unwrap();

        let cmds = engine.drain_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            ScriptCommand::Into { table, format, .. } => {
                assert_eq!(table, "flows");
                assert!(matches!(format, ImportFormat::Csv));
            }
            _ => panic!("expected Into command"),
        }
    }

    #[test]
    fn test_derive_command() {
        let mut engine = ScriptEngine::new();
        engine
            .eval("derive tcp_only {SELECT * FROM flows WHERE proto='tcp'}")
            .unwrap();

        let cmds = engine.drain_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            ScriptCommand::Derive { name, sql } => {
                assert_eq!(name, "tcp_only");
                assert!(sql.contains("proto='tcp'"));
            }
            _ => panic!("expected Derive command"),
        }
    }

    #[test]
    fn test_freeze_and_run() {
        let mut engine = ScriptEngine::new();
        engine.eval("freeze").unwrap();
        engine.eval("run").unwrap();

        let cmds = engine.drain_commands();
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], ScriptCommand::Freeze));
        assert!(matches!(cmds[1], ScriptCommand::Run));
    }

    #[test]
    fn test_export_command() {
        let mut engine = ScriptEngine::new();
        engine.eval("export result -csv -name output.csv").unwrap();

        let cmds = engine.drain_commands();
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            ScriptCommand::Export { data_ref, format, path } => {
                assert_eq!(data_ref, "result");
                assert!(matches!(format, ExportFormat::Csv));
                assert_eq!(path, "output.csv");
            }
            _ => panic!("expected Export command"),
        }
    }
}
