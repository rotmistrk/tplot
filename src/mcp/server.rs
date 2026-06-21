//! JSON-RPC 2.0 MCP server — listens on Unix socket, dispatches tools.

use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{json, Value};

use super::commands::{McpAction, McpCommandQueue};

/// Shared handle for the command queue (set after waker is available).
pub type SharedCommandQueue = Arc<Mutex<Option<McpCommandQueue>>>;

/// Start the MCP listener on the given socket path.
pub fn start_mcp_listener(socket_path: &Path) -> Result<PathBuf, String> {
    if UnixStream::connect(socket_path).is_ok() {
        return Err("tplot already running for this project".into());
    }
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }

    let listener = UnixListener::bind(socket_path)
        .map_err(|e| format!("bind MCP socket: {e}"))?;

    let path = socket_path.to_path_buf();
    let cmd_queue: SharedCommandQueue = Arc::new(Mutex::new(None));
    let cq = Arc::clone(&cmd_queue);

    thread::spawn(move || accept_loop(listener, cq));
    Ok(path)
}

/// Start with an existing command queue (for waker integration).
pub fn start_mcp_listener_with_queue(
    socket_path: &Path,
    cmd_queue: SharedCommandQueue,
) -> Result<PathBuf, String> {
    if UnixStream::connect(socket_path).is_ok() {
        return Err("tplot already running for this project".into());
    }
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }

    let listener = UnixListener::bind(socket_path)
        .map_err(|e| format!("bind MCP socket: {e}"))?;

    let path = socket_path.to_path_buf();
    let cq = Arc::clone(&cmd_queue);
    thread::spawn(move || accept_loop(listener, cq));
    Ok(path)
}

fn accept_loop(listener: UnixListener, cmd_queue: SharedCommandQueue) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { break };
        let cq = Arc::clone(&cmd_queue);
        thread::spawn(move || {
            let _ = handle_connection(stream, cq);
        });
    }
}

fn handle_connection(stream: UnixStream, cmd_queue: SharedCommandQueue) -> io::Result<()> {
    let reader = io::BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            writeln!(writer, "{}", jsonrpc_error(None, -32700, "Parse error"))?;
            writer.flush()?;
            continue;
        };
        let method = request.get("method").and_then(Value::as_str).unwrap_or("?");
        if method == "notifications/initialized" {
            continue;
        }
        let Some(id) = request.get("id") else { continue };
        let response = dispatch(id, method, &request, &cmd_queue);
        writeln!(writer, "{response}")?;
        writer.flush()?;
    }
    Ok(())
}

fn dispatch(id: &Value, method: &str, request: &Value, cmd_queue: &SharedCommandQueue) -> Value {
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
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            handle_tool_call(id, tool_name, &arguments, cmd_queue)
        }
        _ => jsonrpc_error(Some(id), -32601, &format!("Method not found: {method}")),
    }
}

fn handle_tool_call(id: &Value, tool_name: &str, args: &Value, cmd_queue: &SharedCommandQueue) -> Value {
    let cq = cmd_queue.lock().ok().and_then(|g| g.clone());
    let Some(cq) = cq else {
        return tool_error(id, "tplot not ready (no waker)");
    };

    let result = match tool_name {
        "run_command" => {
            let script = args.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if script.is_empty() {
                Err("missing 'command' argument".into())
            } else {
                cq.send(McpAction::RunCommand { script })
            }
        }
        "set_editor_content" => {
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
            cq.send(McpAction::SetEditorContent { content })
        }
        "get_editor_content" => {
            cq.send(McpAction::GetEditorContent)
        }
        "list_nodes" => {
            cq.send(McpAction::ListNodes)
        }
        "preview_table" => {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            if name.is_empty() {
                Err("missing 'name' argument".into())
            } else {
                cq.send(McpAction::PreviewTable { name, limit })
            }
        }
        _ => Err(format!("unknown tool: {tool_name}")),
    };

    match result {
        Ok(val) => {
            let text = if val.is_string() {
                val.as_str().unwrap_or("").to_owned()
            } else {
                serde_json::to_string_pretty(&val).unwrap_or_default()
            };
            let r = json!({"content": [{"type": "text", "text": text}]});
            jsonrpc_result(id, &r)
        }
        Err(msg) => tool_error(id, &msg),
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "run_command",
            "description": "Execute a tplot/Tcl command (same as typing in editor and pressing F9). Examples: sql {SELECT ...}, into tablename --shell {cmd} --csv",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Tcl command to execute"}
                },
                "required": ["command"]
            }
        },
        {
            "name": "list_nodes",
            "description": "List all nodes in the lineage graph with their type, status, and parent relationships",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "preview_table",
            "description": "Preview data from a table/query node (returns first N rows as JSON)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Node/table name"},
                    "limit": {"type": "integer", "description": "Max rows (default 20)"}
                },
                "required": ["name"]
            }
        },
        {
            "name": "get_editor_content",
            "description": "Get the current content of the command editor",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "set_editor_content",
            "description": "Set the command editor content (replaces buffer)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {"type": "string", "description": "New editor content"}
                },
                "required": ["content"]
            }
        }
    ])
}

fn tool_error(id: &Value, msg: &str) -> Value {
    let r = json!({"isError": true, "content": [{"type": "text", "text": msg}]});
    jsonrpc_result(id, &r)
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
