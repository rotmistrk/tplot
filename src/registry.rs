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
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self { nodes: Vec::new() }
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
        self.nodes.push(Node {
            name: name.to_string(),
            parents: vec![],
            behavior,
            status: NodeStatus::UpToDate,
            meta,
        });
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
        self.nodes.push(Node {
            name: name.to_string(),
            parents,
            behavior,
            status: NodeStatus::UpToDate,
            meta,
        });
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
        self.nodes.push(Node {
            name: name.to_string(),
            parents,
            behavior,
            status: NodeStatus::UpToDate,
            meta: NodeMeta::default(),
        });
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

// Table reference detection moved to sql_analysis module.
