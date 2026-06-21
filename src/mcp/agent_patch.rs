//! Agent patching — ensures kiro agent has tplot MCP server configured.
//!
//! On kiro launch, copies the source agent file to `.kiro/agents/tplot-<name>.json`
//! with tplot MCP server patched in.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Ensure a patched agent file exists. Returns the patched agent name.
pub fn ensure_agent_patched(root: &Path, agent_name: &str) -> Result<String, String> {
    if agent_name == "tplot" {
        // Self-agent: generate it if missing.
        let path = root.join(".kiro/agents/tplot.json");
        if !path.is_file() || !file_has_tplot_mcp(&path) {
            let val = serde_json::json!({
                "name": "tplot",
                "tools": ["*"],
                "allowedTools": ["@tplot"],
                "includeMcpJson": true,
                "mcpServers": {"tplot": tplot_mcp_server_def()}
            });
            write_patched(root, &path, &val)?;
        }
        return Ok("tplot".into());
    }

    let home = env::var("HOME").unwrap_or_default();
    let home_dir = Path::new(&home).join(".kiro/agents");
    let source_path = find_agent_file(&home_dir, agent_name);
    let local_dir = root.join(".kiro/agents");
    let local_source = find_agent_file(&local_dir, agent_name);

    if source_path.is_none() && local_source.is_none() {
        return Err(format!("agent '{agent_name}' not found in ~/.kiro/agents/ or .kiro/agents/"));
    }

    let patched_name = format!("tplot-{agent_name}");
    let patched_path = root.join(format!(".kiro/agents/{patched_name}.json"));

    let best_source = source_path.as_deref().or(local_source.as_deref());
    if needs_repatch(&patched_path, best_source) {
        let base = load_source(best_source, agent_name)?;
        let patched = patch_agent(base, &patched_name)?;
        write_patched(root, &patched_path, &patched)?;
    }
    Ok(patched_name)
}

fn find_agent_file(dir: &Path, agent_name: &str) -> Option<PathBuf> {
    let exact = dir.join(format!("{agent_name}.json"));
    if exact.is_file() {
        return Some(exact);
    }
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                if val.get("name").and_then(|n| n.as_str()) == Some(agent_name) {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn needs_repatch(patched: &Path, source: Option<&Path>) -> bool {
    if !patched.is_file() {
        return true;
    }
    if !file_has_tplot_mcp(patched) {
        return true;
    }
    // Check if source is newer
    if let Some(src) = source {
        let src_mtime = fs::metadata(src).and_then(|m| m.modified()).ok();
        let dst_mtime = fs::metadata(patched).and_then(|m| m.modified()).ok();
        if let (Some(s), Some(d)) = (src_mtime, dst_mtime) {
            return s > d;
        }
    }
    false
}

fn file_has_tplot_mcp(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else { return false };
    let Ok(val) = serde_json::from_str::<Value>(&content) else { return false };
    val.get("mcpServers").and_then(|s| s.get("tplot")).is_some()
}

fn load_source(source: Option<&Path>, agent_name: &str) -> Result<Value, String> {
    if let Some(path) = source {
        let content = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        return serde_json::from_str(&content).map_err(|e| format!("parse {}: {e}", path.display()));
    }
    Ok(serde_json::json!({"name": agent_name, "tools": ["*"]}))
}

fn patch_agent(mut val: Value, patched_name: &str) -> Result<Value, String> {
    let obj = val.as_object_mut().ok_or("agent JSON is not an object")?;
    obj.insert("name".to_string(), Value::String(patched_name.to_string()));

    let servers = obj.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let servers_obj = servers.as_object_mut().ok_or("mcpServers is not an object")?;
    servers_obj.insert("tplot".to_string(), tplot_mcp_server_def());

    let allowed = obj.entry("allowedTools").or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = allowed.as_array_mut() {
        let tag = Value::String("@tplot".to_string());
        if !arr.contains(&tag) {
            arr.push(tag);
        }
    }

    let tools = obj.entry("tools").or_insert_with(|| Value::Array(Vec::new()));
    if let Some(arr) = tools.as_array_mut() {
        let tag = Value::String("@tplot".to_string());
        if !arr.contains(&tag) {
            arr.push(tag);
        }
    }

    obj.insert("includeMcpJson".to_string(), Value::Bool(true));
    Ok(val)
}

fn tplot_mcp_server_def() -> Value {
    let bin = env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "tplot".to_owned());
    serde_json::json!({
        "command": bin,
        "args": ["--mcp-server"],
        "env": {"TPLOT_MCP_SOCKET": "${TPLOT_MCP_SOCKET}"}
    })
}

fn write_patched(root: &Path, path: &Path, val: &Value) -> Result<(), String> {
    let dir = root.join(".kiro/agents");
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let json = serde_json::to_string_pretty(val).map_err(|e| format!("serialize: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}
