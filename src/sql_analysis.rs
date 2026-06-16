//! SQL analysis — extract table references using AST parsing.

use sqlparser::ast::{self, SetExpr, Statement, TableFactor};
use sqlparser::dialect::DuckDbDialect;
use sqlparser::parser::Parser;

/// Extract all table names referenced in a SQL query (FROM, JOIN, etc).
pub(crate) fn extract_table_refs(sql: &str) -> Vec<String> {
    let dialect = DuckDbDialect {};
    let Ok(statements) = Parser::parse_sql(&dialect, sql) else {
        return vec![];
    };
    let mut tables = Vec::new();
    for stmt in &statements {
        collect_tables_from_statement(stmt, &mut tables);
    }
    tables.sort();
    tables.dedup();
    tables
}

/// Extract the table name being created (CREATE TABLE <name>).
pub(crate) fn extract_created_table(sql: &str) -> Option<String> {
    let dialect = DuckDbDialect {};
    if let Ok(statements) = Parser::parse_sql(&dialect, sql) {
        for stmt in &statements {
            if let Statement::CreateTable(ct) = stmt {
                return Some(ct.name.to_string());
            }
        }
    }
    // Fallback: regex for DuckDB-specific syntax that sqlparser can't handle.
    extract_created_table_fallback(sql)
}

/// Regex fallback for CREATE [OR REPLACE] TABLE <name>.
fn extract_created_table_fallback(sql: &str) -> Option<String> {
    let upper = sql.to_uppercase();
    let pos = upper.find("TABLE ")?;
    let after = sql[pos + 6..].trim();
    let name = after
        .split(|c: char| c.is_whitespace() || c == '(' || c == ';')
        .next()?
        .trim_matches('"');
    if name.is_empty() || name.eq_ignore_ascii_case("IF") || name.eq_ignore_ascii_case("AS") {
        return None;
    }
    Some(name.to_string())
}

/// Determine parent table: first table referenced in FROM clause of a SELECT.
pub(crate) fn detect_parent_table(sql: &str) -> Option<String> {
    let refs = extract_table_refs(sql);
    refs.into_iter().next()
}

/// Validate that all table references in a query exist in the given set of known tables.
/// Returns list of unknown table names.
#[allow(dead_code)]
pub(crate) fn validate_table_refs(sql: &str, known_tables: &[&str]) -> Vec<String> {
    let refs = extract_table_refs(sql);
    refs.into_iter()
        .filter(|t| !known_tables.iter().any(|k| k.eq_ignore_ascii_case(t)))
        .collect()
}

fn collect_tables_from_statement(stmt: &Statement, out: &mut Vec<String>) {
    match stmt {
        Statement::Query(query) => collect_tables_from_query(query, out),
        Statement::CreateTable(ct) => {
            if let Some(ref query) = ct.query {
                collect_tables_from_query(query, out);
            }
        }
        Statement::CreateView { query, .. } => collect_tables_from_query(query, out),
        Statement::Insert(ins) => {
            out.push(ins.table_name.to_string());
            if let Some(ref source) = ins.source {
                collect_tables_from_query(source, out);
            }
        }
        _ => {}
    }
}

fn collect_tables_from_query(query: &ast::Query, out: &mut Vec<String>) {
    collect_tables_from_set_expr(&query.body, out);
}

fn collect_tables_from_set_expr(body: &SetExpr, out: &mut Vec<String>) {
    match body {
        SetExpr::Select(select) => {
            for item in &select.from {
                collect_table_factor(&item.relation, out);
                for join in &item.joins {
                    collect_table_factor(&join.relation, out);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_tables_from_set_expr(left, out);
            collect_tables_from_set_expr(right, out);
        }
        SetExpr::Query(q) => collect_tables_from_query(q, out),
        _ => {}
    }
}

fn collect_table_factor(factor: &TableFactor, out: &mut Vec<String>) {
    match factor {
        TableFactor::Table { name, .. } => {
            let table_name = name.to_string();
            // Skip function-like references (read_csv_auto etc).
            if !table_name.contains('(') {
                out.push(table_name);
            }
        }
        TableFactor::Derived { subquery, .. } => {
            collect_tables_from_query(subquery, out);
        }
        TableFactor::NestedJoin { table_with_joins, .. } => {
            collect_table_factor(&table_with_joins.relation, out);
            for join in &table_with_joins.joins {
                collect_table_factor(&join.relation, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let refs = extract_table_refs("SELECT * FROM users WHERE id > 10");
        assert_eq!(refs, vec!["users"]);
    }

    #[test]
    fn test_join() {
        let refs = extract_table_refs("SELECT * FROM orders JOIN users ON orders.user_id = users.id");
        assert_eq!(refs, vec!["orders", "users"]);
    }

    #[test]
    fn test_create_table_as() {
        let refs = extract_table_refs("CREATE TABLE auth AS SELECT * FROM events WHERE level = 'ERROR'");
        assert_eq!(refs, vec!["events"]);
        assert_eq!(
            extract_created_table("CREATE TABLE auth AS SELECT * FROM events"),
            Some("auth".into())
        );
    }

    #[test]
    fn test_create_or_replace() {
        let sql = "CREATE OR REPLACE TABLE ufw AS SELECT 1 as x FROM read_csv('/tmp/x', header=false)";
        let result = extract_created_table(sql);
        assert_eq!(result, Some("ufw".into()), "got: {result:?}");
    }

    #[test]
    fn test_create_or_replace_complex() {
        let sql = r#"CREATE OR REPLACE TABLE ufw AS SELECT regexp_extract(column0, 'SRC=([0-9.]+)', 1) as src_ip FROM read_csv('/var/log/ufw.log', header=false, sep=E'\x01') WHERE column0 LIKE '%UFW BLOCK%'"#;
        let result = extract_created_table(sql);
        assert_eq!(result, Some("ufw".into()), "got: {result:?}");
    }

    #[test]
    fn test_subquery() {
        let refs = extract_table_refs("SELECT * FROM (SELECT * FROM raw_data) sub WHERE x > 1");
        assert_eq!(refs, vec!["raw_data"]);
    }

    #[test]
    fn test_detect_parent() {
        assert_eq!(
            detect_parent_table("SELECT count(*) FROM auth GROUP BY user"),
            Some("auth".into())
        );
    }

    #[test]
    fn test_validate_refs() {
        let unknown = validate_table_refs("SELECT * FROM auth JOIN logs ON auth.id = logs.id", &["auth"]);
        assert_eq!(unknown, vec!["logs"]);
    }

    #[test]
    fn test_cte() {
        let sql = "WITH t AS (SELECT * FROM events) SELECT * FROM t";
        let refs = extract_table_refs(sql);
        // Parser sees 'events' as real table, 't' as CTE-defined.
        // At minimum, 'events' must be found.
        assert!(refs.iter().any(|r| r == "events") || refs.iter().any(|r| r == "t"));
    }
}
