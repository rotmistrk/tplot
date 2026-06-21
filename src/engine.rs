//! Data engine — DuckDB integration for query execution and data management.

use std::path::Path;
use std::sync::mpsc;
use std::thread;

use duckdb::arrow::array::Array;
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::Connection;

/// Result set from a query: column names + rows of string values.
#[allow(dead_code)]
pub(crate) struct QueryResult {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) row_count: usize,
}

/// Progress message from async operations.
#[allow(dead_code)]
pub(crate) enum Progress {
    Started { task: String },
    Update { rows_processed: usize },
    Done { result: Result<QueryResult, String> },
}

/// The data engine wraps a DuckDB connection.
#[allow(dead_code)]
pub(crate) struct Engine {
    conn: Connection,
}

#[allow(dead_code)]
impl Engine {
    /// Open or create a DuckDB database in the project directory.
    pub(crate) fn open(project_dir: &Path) -> Result<Self, String> {
        let db_path = project_dir.join(".tplot").join("tplot.duckdb");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
        }
        let conn = Connection::open(&db_path).map_err(|e| format!("duckdb open: {e}"))?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for testing).
    pub(crate) fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("duckdb memory: {e}"))?;
        Ok(Self { conn })
    }

    /// Execute a SQL query and return results via Arrow.
    pub(crate) fn query(&self, sql: &str) -> Result<QueryResult, String> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| format!("prepare: {e}"))?;
        let batches: Vec<RecordBatch> = stmt.query_arrow([]).map_err(|e| format!("query: {e}"))?.collect();

        if batches.is_empty() {
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                row_count: 0,
            });
        }

        let schema = batches[0].schema();
        let columns: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

        let mut rows = Vec::new();
        for batch in &batches {
            for row_idx in 0..batch.num_rows() {
                let mut row = Vec::with_capacity(columns.len());
                for col_idx in 0..batch.num_columns() {
                    let col = batch.column(col_idx);
                    row.push(arrow_value_to_string(col, row_idx));
                }
                rows.push(row);
            }
        }
        let row_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            row_count,
        })
    }

    /// Import a CSV file into a named table (blocking).
    pub(crate) fn import_csv(&self, path: &Path, table_name: &str) -> Result<QueryResult, String> {
        let path_str = path.to_string_lossy();
        let sql = format!("CREATE OR REPLACE TABLE \"{table_name}\" AS SELECT * FROM read_csv_auto('{path_str}')");
        self.conn.execute_batch(&sql).map_err(|e| format!("import: {e}"))?;
        let count_sql = format!("SELECT count(*) as rows FROM \"{table_name}\"");
        self.query(&count_sql)
    }

    /// Import CSV on a background thread, reporting progress.
    pub(crate) fn import_csv_async(project_dir: &Path, csv_path: &Path, table_name: &str) -> mpsc::Receiver<Progress> {
        let (tx, rx) = mpsc::channel();
        let project_dir = project_dir.to_path_buf();
        let csv_path = csv_path.to_path_buf();
        let table_name = table_name.to_string();

        thread::spawn(move || {
            let _ = tx.send(Progress::Started {
                task: format!("Importing {}", csv_path.display()),
            });
            let result = (|| {
                let engine = Engine::open(&project_dir)?;
                engine.import_csv(&csv_path, &table_name)
            })();
            let _ = tx.send(Progress::Done { result });
        });
        rx
    }

    /// Spawn shell import job. Returns (JobHandle, CancelToken).
    pub(crate) fn spawn_shell_import(
        project_dir: &Path,
        cmd: &str,
        table_name: &str,
    ) -> (crate::jobs::JobHandle, crate::jobs::CancelToken) {
        use crate::jobs::{new_cancel_token, JobHandle, Progress as JP};
        use std::time::Instant;

        let (tx, rx) = mpsc::channel();
        let cancel = new_cancel_token();
        let cancel_clone = cancel.clone();
        let project_dir = project_dir.to_path_buf();
        let cmd = cmd.to_string();
        let table_name = table_name.to_string();
        let node_id = table_name.clone();

        thread::spawn(move || {
            let _ = tx.send(JP::Started { task: format!("shell: {cmd}") });

            // Run the shell command, capture stdout
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            let mut child = match output {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(JP::Done { result: Err(format!("spawn: {e}")) });
                    return;
                }
            };

            // Read stdout in chunks, write to temp file, report progress
            use std::io::{BufRead, BufReader, Write};
            let stdout = child.stdout.take().unwrap();
            let reader = BufReader::new(stdout);

            let tmp_path = project_dir.join(format!(".tplot/.import_{table_name}.csv"));
            let tmp_file = match std::fs::File::create(&tmp_path) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(JP::Done { result: Err(format!("tmp file: {e}")) });
                    return;
                }
            };
            let mut writer = std::io::BufWriter::new(tmp_file);
            let mut rows: u64 = 0;
            let mut bytes: u64 = 0;

            for line in reader.lines() {
                if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = child.kill();
                    let _ = std::fs::remove_file(&tmp_path);
                    let _ = tx.send(JP::Cancelled);
                    return;
                }
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                bytes += line.len() as u64 + 1;
                let _ = writeln!(writer, "{}", line);
                rows += 1;
                if rows % 1000 == 0 {
                    let _ = tx.send(JP::Update { rows_done: rows, rows_total: None, bytes_done: bytes });
                }
            }
            drop(writer);
            let _ = child.wait();

            let _ = tx.send(JP::Update { rows_done: rows, rows_total: Some(rows), bytes_done: bytes });

            // Load into DuckDB
            let result = (|| -> Result<String, String> {
                let engine = Engine::open(&project_dir)?;
                let path_str = tmp_path.to_string_lossy();
                let sql = format!(
                    "CREATE OR REPLACE TABLE \"{table_name}\" AS SELECT * FROM read_csv_auto('{path_str}')"
                );
                engine.conn.execute_batch(&sql).map_err(|e| format!("import: {e}"))?;
                let _ = std::fs::remove_file(&tmp_path);
                Ok(format!("{rows} rows imported"))
            })();

            let _ = tx.send(JP::Done { result });
        });

        let handle = JobHandle {
            node_id,
            task: format!("shell import"),
            rx,
            cancel: cancel.clone(),
            started_at: Instant::now(),
            rows_done: 0,
            rows_total: None,
            bytes_done: 0,
        };
        (handle, cancel)
    }
}

/// Convert an Arrow array cell to a display string.
fn arrow_value_to_string(col: &dyn Array, idx: usize) -> String {
    use duckdb::arrow::array::{Float32Array, Float64Array, Int32Array, Int64Array, StringArray};

    if col.is_null(idx) {
        return "NULL".to_string();
    }
    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
        return arr.value(idx).to_string();
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
        return arr.value(idx).to_string();
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return arr.value(idx).to_string();
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
        return arr.value(idx).to_string();
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
        return arr.value(idx).to_string();
    }
    // Fallback: use debug display
    format!("{:?}", col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_query_basic() {
        let engine = Engine::open_memory().unwrap();
        let result = engine.query("SELECT 1 as x, 'hello' as y").unwrap();
        assert_eq!(result.columns, vec!["x", "y"]);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][1], "hello");
    }

    #[test]
    fn test_import_csv() {
        let engine = Engine::open_memory().unwrap();
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "name,value").unwrap();
        writeln!(f, "alpha,10").unwrap();
        writeln!(f, "beta,20").unwrap();
        writeln!(f, "gamma,30").unwrap();
        f.flush().unwrap();

        let result = engine.import_csv(f.path(), "test_data").unwrap();
        assert_eq!(result.row_count, 1); // count query returns 1 row
        assert_eq!(result.rows[0][0], "3"); // 3 rows imported

        let data = engine.query("SELECT * FROM test_data ORDER BY name").unwrap();
        assert_eq!(data.columns, vec!["name", "value"]);
        assert_eq!(data.row_count, 3);
        assert_eq!(data.rows[0][0], "alpha");
    }

    #[test]
    fn test_import_csv_async() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("data.csv");
        std::fs::write(&csv_path, "x,y\n1,2\n3,4\n").unwrap();

        let rx = Engine::import_csv_async(dir.path(), &csv_path, "async_test");

        let mut got_started = false;
        let mut got_done = false;
        for msg in rx {
            match msg {
                Progress::Started { .. } => got_started = true,
                Progress::Done { result } => {
                    got_done = true;
                    assert!(result.is_ok());
                }
                Progress::Update { .. } => {}
            }
        }
        assert!(got_started);
        assert!(got_done);
    }
}
