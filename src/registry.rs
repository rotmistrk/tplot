//! Node registry v2 — trait-based polymorphic nodes.

use std::time::SystemTime;

use crate::engine::Engine;
use crate::node_behavior::{NodeBehavior, NodeResult, PlotNode, QueryNode, TableNode};
use crate::node_state::{NodeMeta, NodeStatus};

/// A node in the lineage graph.
#[allow(dead_code)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) parents: Vec<String>,
    pub(crate) behavior: Box<dyn NodeBehavior>,
    pub(crate) status: NodeStatus,
    pub(crate) meta: NodeMeta,
}

impl Node {
    /// Execute this node and update metadata.
    pub(crate) fn execute(&self, engine: &Engine) -> Result<NodeResult, String> {
        self.behavior.execute(engine)
    }

    pub(crate) fn icon(&self) -> &str {
        self.behavior.icon()
    }

    pub(crate) fn command(&self) -> &str {
        self.behavior.command()
    }
}

/// Manages the lineage graph.
pub(crate) struct Registry {
    nodes: Vec<Node>,
    nodes_dir: Option<std::path::PathBuf>,
}

impl Registry {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            nodes_dir: None,
        }
    }

    /// Create registry backed by a project directory. Loads existing nodes.
    pub(crate) fn open(project_dir: &std::path::Path) -> Self {
        let nodes_dir = project_dir.join("nodes");
        let _ = std::fs::create_dir_all(&nodes_dir);
        let nodes = load_all_nodes(&nodes_dir);
        Self {
            nodes,
            nodes_dir: Some(nodes_dir),
        }
    }

    pub(crate) fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Find a node by name.
    pub(crate) fn find(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.name == name)
    }

    /// Add a materialized table node.
    pub(crate) fn add_table(&mut self, name: &str, cmd: &str, create_sql: &str, row_count: Option<u64>) {
        self.remove_by_name(name);
        let behavior = Box::new(TableNode {
            table_name: name.to_string(),
            cmd: cmd.to_string(),
            create_sql: create_sql.to_string(),
        });
        let meta = NodeMeta {
            row_count,
            last_run_at: Some(SystemTime::now()),
            ..NodeMeta::default()
        };
        let node = Node {
            name: name.to_string(),
            parents: vec![],
            behavior,
            status: NodeStatus::UpToDate,
            meta,
        };
        self.persist(&node);
        self.nodes.push(node);
    }

    /// Add a query node.
    pub(crate) fn add_query(&mut self, name: &str, cmd: &str, sql: &str, parent: Option<&str>, row_count: Option<u64>) {
        self.remove_by_name(name);
        let behavior = Box::new(QueryNode {
            cmd: cmd.to_string(),
            sql: sql.to_string(),
        });
        let meta = NodeMeta {
            row_count,
            last_run_at: Some(SystemTime::now()),
            ..NodeMeta::default()
        };
        let parents = parent.map(|p| vec![p.to_string()]).unwrap_or_default();
        let node = Node {
            name: name.to_string(),
            parents,
            behavior,
            status: NodeStatus::UpToDate,
            meta,
        };
        self.persist(&node);
        self.nodes.push(node);
    }

    /// Add a plot node.
    pub(crate) fn add_plot(&mut self, name: &str, cmd: &str, plot_type: &str, data_source: &str, columns: &[String]) {
        self.remove_by_name(name);
        let behavior = Box::new(PlotNode {
            cmd: cmd.to_string(),
            plot_type: plot_type.to_string(),
            data_source: data_source.to_string(),
            columns: columns.to_vec(),
        });
        let parents = vec![data_source.to_string()];
        let node = Node {
            name: name.to_string(),
            parents,
            behavior,
            status: NodeStatus::UpToDate,
            meta: NodeMeta::default(),
        };
        self.persist(&node);
        self.nodes.push(node);
    }

    /// Mark all children of a node as Dirty.
    #[allow(dead_code)]
    pub(crate) fn mark_children_dirty(&mut self, parent_name: &str) {
        for node in &mut self.nodes {
            if node.parents.iter().any(|p| p == parent_name) {
                node.status = NodeStatus::Dirty;
            }
        }
    }

    fn remove_by_name(&mut self, name: &str) {
        self.nodes.retain(|n| n.name != name);
    }

    fn persist(&self, node: &Node) {
        if let Some(ref dir) = self.nodes_dir {
            save_node(dir, node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_find() {
        let mut reg = Registry::new();
        reg.add_table(
            "auth",
            "sql {CREATE TABLE auth...}",
            "CREATE TABLE auth AS SELECT 1",
            Some(5),
        );
        reg.add_query(
            "top",
            "sql -name top {SELECT..}",
            "SELECT * FROM auth LIMIT 5",
            Some("auth"),
            Some(5),
        );
        reg.add_plot("chart", "plot bar top x y", "bar", "top", &["x".into(), "y".into()]);

        assert_eq!(reg.nodes().len(), 3);
        assert_eq!(reg.find("auth").unwrap().icon(), "[T]");
        assert_eq!(reg.find("top").unwrap().icon(), "[Q]");
        assert_eq!(reg.find("chart").unwrap().icon(), "[P]");
        assert_eq!(reg.find("top").unwrap().parents, vec!["auth"]);
    }

    #[test]
    fn test_mark_dirty() {
        let mut reg = Registry::new();
        reg.add_table("auth", "cmd", "", None);
        reg.add_query("q1", "cmd", "SELECT 1", Some("auth"), None);
        reg.add_query("q2", "cmd", "SELECT 2", Some("auth"), None);

        reg.mark_children_dirty("auth");
        assert_eq!(reg.find("q1").unwrap().status, NodeStatus::Dirty);
        assert_eq!(reg.find("q2").unwrap().status, NodeStatus::Dirty);
        assert_eq!(reg.find("auth").unwrap().status, NodeStatus::UpToDate);
    }

    #[test]
    fn test_execute_through_trait() {
        let engine = Engine::open_memory().unwrap();
        engine.query("CREATE TABLE t AS SELECT 1 as x").unwrap();

        let mut reg = Registry::new();
        reg.add_query("q", "cmd", "SELECT * FROM t", None, None);

        let node = reg.find("q").unwrap();
        let result = node.execute(&engine).unwrap();
        match result {
            NodeResult::Table(qr) => assert_eq!(qr.row_count, 1),
            _ => panic!("expected Table"),
        }
    }
}

// ─── Persistence ────────────────────────────────────────────────────────

/// Save a node as node.tcl. File name = node name (sanitized).
fn save_node(nodes_dir: &std::path::Path, node: &Node) {
    let file_name = sanitize_name(&node.name);
    let path = nodes_dir.join(format!("{file_name}.tcl"));
    let mut content = String::new();
    content.push_str(&format!("# node: {}\n", node.name));
    if !node.parents.is_empty() {
        content.push_str(&format!("# parent: {}\n", node.parents.join(", ")));
    }
    content.push_str(&format!("# icon: {}\n", node.icon()));
    if let Some(count) = node.meta.row_count {
        content.push_str(&format!("# rows: {count}\n"));
    }
    content.push('\n');
    content.push_str(node.command());
    content.push('\n');
    let _ = std::fs::write(path, content);
}

/// Load all nodes from .tcl files in the nodes directory.
fn load_all_nodes(nodes_dir: &std::path::Path) -> Vec<Node> {
    let Ok(entries) = std::fs::read_dir(nodes_dir) else {
        return vec![];
    };
    let mut nodes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("tcl") {
            continue;
        }
        if let Some(node) = parse_node_file(&path) {
            nodes.push(node);
        }
    }
    nodes
}

fn parse_node_file(path: &std::path::Path) -> Option<Node> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut parents = Vec::new();
    let mut icon = String::new();
    let mut row_count = None;
    let mut command_lines = Vec::new();

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("# node: ") {
            name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("# parent: ") {
            parents = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(v) = line.strip_prefix("# icon: ") {
            icon = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("# rows: ") {
            row_count = v.trim().parse().ok();
        } else if !line.starts_with('#') && !line.is_empty() {
            command_lines.push(line.to_string());
        }
    }

    if name.is_empty() {
        return None;
    }

    let cmd = command_lines.join("\n");
    let behavior: Box<dyn NodeBehavior> = match icon.as_str() {
        "[T]" => Box::new(TableNode {
            table_name: name.clone(),
            cmd: cmd.clone(),
            create_sql: cmd.clone(),
        }),
        "[P]" => {
            // Parse plot command: "plot <type> <source> <col1> <col2>..."
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            Box::new(PlotNode {
                cmd: cmd.clone(),
                plot_type: parts.get(1).unwrap_or(&"bar").to_string(),
                data_source: parts.get(2).unwrap_or(&"").to_string(),
                columns: parts.iter().skip(3).map(|s| s.to_string()).collect(),
            })
        }
        _ => {
            // Default: query node. Extract SQL from command.
            let sql = if let Some(rest) = cmd.strip_prefix("sql -name ") {
                rest.find('{')
                    .map(|i| &rest[i + 1..rest.len() - 1])
                    .unwrap_or("")
                    .to_string()
            } else if let Some(rest) = cmd.strip_prefix("sql {") {
                rest.trim_end_matches('}').to_string()
            } else if let Some(rest) = cmd.strip_prefix("derive ") {
                rest.find('{')
                    .map(|i| &rest[i + 1..rest.len() - 1])
                    .unwrap_or("")
                    .to_string()
            } else {
                cmd.clone()
            };
            Box::new(QueryNode { cmd: cmd.clone(), sql })
        }
    };

    let meta = NodeMeta {
        row_count,
        ..NodeMeta::default()
    };
    Some(Node {
        name,
        parents,
        behavior,
        status: NodeStatus::UpToDate,
        meta,
    })
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

// Table reference detection moved to sql_analysis module.
