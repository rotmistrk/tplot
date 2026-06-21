//! JSON-RPC 2.0 MCP server — listens on Unix socket.

use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;

use serde_json::{json, Value};

/// Start the MCP listener on the given socket path. Returns the path on success.
pub fn start_mcp_listener(socket_path: &Path) -> Result<PathBuf, String> {
    // Clean up stale socket
    if UnixStream::connect(socket_path).is_ok() {
        return Err("tplot already running for this project".into());
    }
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }

    let listener = UnixListener::bind(socket_path)
        .map_err(|e| format!("bind MCP socket: {e}"))?;

    let path = socket_path.to_path_buf();
    thread::spawn(move || accept_loop(listener));
    Ok(path)
}

fn accept_loop(listener: UnixListener) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { break };
        thread::spawn(move || {
            let _ = handle_connection(stream);
        });
    }
}

fn handle_connection(stream: UnixStream) -> io::Result<()> {
    let reader = io::BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            let err = jsonrpc_error(None, -32700, "Parse error");
            writeln!(writer, "{err}")?;
            writer.flush()?;
            continue;
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("?");
        if method == "notifications/initialized" {
            continue;
        }
        let Some(id) = request.get("id") else { continue };
        let response = dispatch(id, method, &request);
        writeln!(writer, "{response}")?;
        writer.flush()?;
    }
    Ok(())
}

fn dispatch(id: &Value, method: &str, _request: &Value) -> Value {
    match method {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "tplot", "version": "0.1.0"},
            });
            jsonrpc_result(id, &result)
        }
        "tools/list" => {
            let result = json!({"tools": tool_definitions()});
            jsonrpc_result(id, &result)
        }
        "tools/call" => {
            let params = _request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let text = format!("Tool '{tool_name}' not yet implemented");
            let r = json!({"isError": true, "content": [{"type": "text", "text": text}]});
            jsonrpc_result(id, &r)
        }
        _ => jsonrpc_error(Some(id), -32601, &format!("Method not found: {method}")),
    }
}

fn tool_definitions() -> Value {
    json!([])
}

fn jsonrpc_error(id: Option<&Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned().unwrap_or(Value::Null),
        "error": {"code": code, "message": message},
    })
}

fn jsonrpc_result(id: &Value, result: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}
