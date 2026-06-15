//! NodeBehavior trait — polymorphic node execution.
//! Each node type implements this trait. Adding a new type = implement trait = compiler enforces.

use crate::engine::{Engine, QueryResult};

/// What a node produces when executed.
pub(crate) enum NodeResult {
    /// Tabular data (from sql, into, derive).
    Table(QueryResult),
    /// Rendered plot lines (text-based chart).
    Plot(Vec<String>),
    /// No displayable output (e.g., CREATE TABLE side effect).
    Nothing,
}

/// Polymorphic node behavior. Each node kind implements this.
pub(crate) trait NodeBehavior: Send + Sync {
    /// Icon for tree display.
    fn icon(&self) -> &str;

    /// Execute this node: produce its result.
    fn execute(&self, engine: &Engine) -> Result<NodeResult, String>;

    /// The tplot command that produces this node (for node.tcl serialization).
    fn command(&self) -> &str;

    /// Can this node have children derived from it?
    fn allows_children(&self) -> bool {
        true
    }
}

// ─── Concrete node types ───────────────────────────────────────────────

/// A materialized table (from `into` or `CREATE TABLE`).
pub(crate) struct TableNode {
    pub(crate) table_name: String,
    pub(crate) cmd: String,
    pub(crate) create_sql: String,
}

impl NodeBehavior for TableNode {
    fn icon(&self) -> &str {
        "[T]"
    }

    fn execute(&self, engine: &Engine) -> Result<NodeResult, String> {
        // If create_sql is non-empty, run it (idempotent due to CREATE OR REPLACE).
        if !self.create_sql.is_empty() {
            engine.query(&self.create_sql)?;
        }
        let result = engine.query(&format!("SELECT * FROM \"{}\" LIMIT 1000", self.table_name))?;
        Ok(NodeResult::Table(result))
    }

    fn command(&self) -> &str {
        &self.cmd
    }
}

/// A query/view node (from `sql -name`).
pub(crate) struct QueryNode {
    pub(crate) cmd: String,
    pub(crate) sql: String,
}

impl NodeBehavior for QueryNode {
    fn icon(&self) -> &str {
        "[Q]"
    }

    fn execute(&self, engine: &Engine) -> Result<NodeResult, String> {
        let result = engine.query(&self.sql)?;
        Ok(NodeResult::Table(result))
    }

    fn command(&self) -> &str {
        &self.cmd
    }
}

/// A plot node.
pub(crate) struct PlotNode {
    pub(crate) cmd: String,
    pub(crate) plot_type: String,
    pub(crate) data_source: String,
    pub(crate) columns: Vec<String>,
}

impl NodeBehavior for PlotNode {
    fn icon(&self) -> &str {
        "[P]"
    }

    fn execute(&self, engine: &Engine) -> Result<NodeResult, String> {
        use crate::plot::{self, Series};

        let sql = if self.columns.len() >= 2 {
            let y_list = self.columns[1..].join(", ");
            format!(
                "SELECT \"{}\", {} FROM \"{}\" LIMIT 500",
                self.columns[0], y_list, self.data_source
            )
        } else {
            format!("SELECT * FROM \"{}\" LIMIT 500", self.data_source)
        };

        // Try direct table, fallback handled by caller.
        let result = engine.query(&sql)?;
        if result.columns.len() < 2 {
            return Err("Need at least 2 columns".into());
        }

        let x_labels: Vec<String> = result.rows.iter().map(|r| r[0].clone()).collect();
        let mut all_series = Vec::new();
        for col_idx in 1..result.columns.len() {
            let values: Vec<(String, f64)> = x_labels
                .iter()
                .zip(result.rows.iter())
                .map(|(lbl, row)| {
                    let val = row.get(col_idx).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    (lbl.clone(), val)
                })
                .collect();
            all_series.push(Series {
                label: result.columns[col_idx].clone(),
                values,
            });
        }

        let lines = match self.plot_type.as_str() {
            "line" => plot::line_chart(&all_series, 80, 20),
            _ => plot::bar_chart(&all_series, 80),
        };

        Ok(NodeResult::Plot(lines))
    }

    fn command(&self) -> &str {
        &self.cmd
    }

    fn allows_children(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_node_execute() {
        let engine = Engine::open_memory().unwrap();
        engine.query("CREATE TABLE t AS SELECT 1 as x, 2 as y").unwrap();

        let node = QueryNode {
            cmd: "sql {SELECT * FROM t}".into(),
            sql: "SELECT * FROM t".into(),
        };
        let result = node.execute(&engine).unwrap();
        match result {
            NodeResult::Table(qr) => {
                assert_eq!(qr.row_count, 1);
                assert_eq!(qr.columns, vec!["x", "y"]);
            }
            _ => panic!("expected Table"),
        }
    }

    #[test]
    fn test_table_node_execute() {
        let engine = Engine::open_memory().unwrap();
        let node = TableNode {
            table_name: "t".into(),
            cmd: "sql {CREATE TABLE t ...}".into(),
            create_sql: "CREATE TABLE t AS SELECT 1 as a, 'hello' as b".into(),
        };
        let result = node.execute(&engine).unwrap();
        match result {
            NodeResult::Table(qr) => assert_eq!(qr.row_count, 1),
            _ => panic!("expected Table"),
        }
    }

    #[test]
    fn test_plot_node_execute() {
        let engine = Engine::open_memory().unwrap();
        engine
            .query("CREATE TABLE d AS SELECT * FROM (VALUES ('a',1),('b',2),('c',3)) AS t(x,y)")
            .unwrap();

        let node = PlotNode {
            cmd: "plot bar d x y".into(),
            plot_type: "bar".into(),
            data_source: "d".into(),
            columns: vec!["x".into(), "y".into()],
        };
        let result = node.execute(&engine).unwrap();
        match result {
            NodeResult::Plot(lines) => assert!(!lines.is_empty()),
            _ => panic!("expected Plot"),
        }
    }
}
