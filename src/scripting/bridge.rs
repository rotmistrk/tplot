//! Bridge commands: registers tplot Tcl commands that produce ScriptCommands.

use std::sync::{Arc, Mutex};

use rusticle::error::TclError;
use rusticle::interpreter::Interpreter;
use rusticle::value::TclValue;

use super::{ExportFormat, ImportFormat, ImportSource, ScriptCommand};

pub fn register(interp: &mut Interpreter, commands: Arc<Mutex<Vec<ScriptCommand>>>) {
    register_sql(interp, commands.clone());
    register_into(interp, commands.clone());
    register_plot(interp, commands.clone());
    register_derive(interp, commands.clone());
    register_export(interp, commands.clone());
    register_freeze(interp, commands.clone());
    register_run(interp, commands);
}

fn push(cmds: &Arc<Mutex<Vec<ScriptCommand>>>, cmd: ScriptCommand) {
    if let Ok(mut guard) = cmds.lock() {
        guard.push(cmd);
    }
}

fn arg_str(args: &[TclValue], idx: usize) -> Result<String, TclError> {
    args.get(idx)
        .map(|v| v.to_string())
        .ok_or_else(|| TclError::new(format!("missing argument {idx}")))
}

fn find_flag(args: &[TclValue], flag: &str) -> Option<usize> {
    args.iter().position(|a| a.to_string() == flag)
}

fn flag_value(args: &[TclValue], flag: &str) -> Option<String> {
    let pos = find_flag(args, flag)?;
    args.get(pos + 1).map(|v| v.to_string())
}

fn register_sql(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("sql", move |_interp, args| {
        let query = arg_str(args, 0)?;
        push(
            &cmds,
            ScriptCommand::Sql {
                query: query.clone(),
                var_name: None,
            },
        );
        Ok(TclValue::from(query))
    });
}

fn register_into(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("into", move |_interp, args| {
        let table = arg_str(args, 0)?;

        let source = if let Some(path) = flag_value(args, "-file") {
            ImportSource::File(path)
        } else {
            let src = arg_str(args, 1)?;
            ImportSource::Exec(src)
        };

        let format = if find_flag(args, "-csv").is_some() {
            ImportFormat::Csv
        } else if find_flag(args, "-tsv").is_some() {
            ImportFormat::Tsv
        } else if find_flag(args, "-json").is_some() {
            ImportFormat::Json
        } else if let Some(sep) = flag_value(args, "-sep") {
            ImportFormat::Sep(sep)
        } else if let Some(pattern) = flag_value(args, "-regex") {
            let cols = flag_value(args, "-cols")
                .unwrap_or_default()
                .split_whitespace()
                .map(String::from)
                .collect();
            ImportFormat::Regex { pattern, cols }
        } else {
            ImportFormat::Auto
        };

        push(
            &cmds,
            ScriptCommand::Into {
                table: table.clone(),
                source,
                format,
            },
        );
        Ok(TclValue::from(table))
    });
}

fn register_plot(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("plot", move |_interp, args| {
        let plot_type = arg_str(args, 0)?;
        let data_ref = arg_str(args, 1)?;
        let options: Vec<String> = args.iter().skip(2).map(|a| a.to_string()).collect();
        push(
            &cmds,
            ScriptCommand::Plot {
                plot_type,
                data_ref,
                options,
            },
        );
        Ok(TclValue::from(""))
    });
}

fn register_derive(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("derive", move |_interp, args| {
        let name = arg_str(args, 0)?;
        let sql = arg_str(args, 1)?;
        push(
            &cmds,
            ScriptCommand::Derive {
                name: name.clone(),
                sql,
            },
        );
        Ok(TclValue::from(name))
    });
}

fn register_export(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("export", move |_interp, args| {
        let data_ref = arg_str(args, 0)?;
        let path = flag_value(args, "-name").ok_or_else(|| TclError::new("-name required".to_string()))?;

        let format = if find_flag(args, "-csv").is_some() {
            ExportFormat::Csv
        } else if find_flag(args, "-jsonl").is_some() {
            ExportFormat::JsonL
        } else if find_flag(args, "-parquet").is_some() {
            ExportFormat::Parquet
        } else if find_flag(args, "-png").is_some() {
            ExportFormat::Png
        } else if find_flag(args, "-svg").is_some() {
            ExportFormat::Svg
        } else {
            ExportFormat::Csv
        };

        push(&cmds, ScriptCommand::Export { data_ref, format, path });
        Ok(TclValue::from(""))
    });
}

fn register_freeze(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("freeze", move |_interp, _args| {
        push(&cmds, ScriptCommand::Freeze);
        Ok(TclValue::from(""))
    });
}

fn register_run(interp: &mut Interpreter, cmds: Arc<Mutex<Vec<ScriptCommand>>>) {
    interp.register_fn("run", move |_interp, _args| {
        push(&cmds, ScriptCommand::Run);
        Ok(TclValue::from(""))
    });
}
