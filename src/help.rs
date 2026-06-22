//! Help text — comprehensive command reference.

pub(crate) fn help_text() -> String {
    "\
═══ tplot — Terminal Data Analysis with Lineage Tracking ═══

QUICK START
  sql {CREATE TABLE auth AS SELECT 'root' as user, '10.0.0.1' as ip, 3 as attempts}
  sql -name top {SELECT * FROM auth ORDER BY attempts DESC}
  into logs --shell {grep Failed /var/log/auth.log} --csv
  plot bar top user attempts

NAVIGATION
  F1          Help (this screen)
  F2          Focus lineage tree
  F3          Focus main panel
  F4          Focus tools panel (cmd editor)
  F5          Zoom focused panel
  F9          Execute current command (at cursor)
  F10         Execute entire buffer
  Ctrl-Q      Quit
  Alt-x       Command line (M-x)

M-x COMMANDS (Alt-x, then type)
  shell                     Open PTY shell in tools panel
  kiro [--agent=name] [flags]  Launch AI agent with MCP connection
  help                      Show this help
  quit / q                  Exit tplot
  <any Tcl>                 Evaluated directly (sql, into, plot, etc.)

LINEAGE TREE (F2 panel)
  j/k         Navigate up/down
  Enter       Execute node (show data in main panel)
  →           Execute + focus main panel
  M-e         Copy node's command to editor
  M-d         Delete node and descendants (with confirmation)
  M-c         Clone node and descendants (appends _copy)

CMD EDITOR (F4 panel, vi modes)
  Ctrl-N      Completion dropdown (commands + table names)
  i/a/o       Enter insert mode
  Esc         Back to normal mode
  :w          Save (session auto-saves on exit)
  :q          Close tab

TCL COMMANDS
  sql {<query>}                         Execute SQL, display result
  sql -name <table> {<query>}           Execute + create named node
  into <table> -file <path>             Import CSV/TSV/JSON file
  into <table> --shell {<cmd>} --csv    Import shell command output (async)
  plot <type> <table> <col1> <col2>     Render chart (bar, line)
  derive <name> {<sql>}                 Create derived query node
  shell                                 Open terminal
  kiro ?args?                           Launch AI agent
  freeze                                Seal current node
  run                                   Re-execute current node

SQL DIALECT (DuckDB)
  Full DuckDB SQL including:
  - CREATE TABLE ... AS SELECT ...
  - CREATE OR REPLACE VIEW ...
  - Window functions, CTEs, UNNEST
  - read_csv_auto(), read_parquet(), read_json()
  - Aggregate functions, QUALIFY, PIVOT

IMPORT OPTIONS
  -file <path>              Read from file (auto-detect format)
  --shell {<command>}       Pipe command stdout (async, parallel ingest)
  --csv                     Force CSV parsing
  --tsv                     Force TSV parsing
  --json                    JSON lines format
  -sep <char>               Custom separator

ASYNC EXECUTION
  Shell imports run in background:
  - Node shows > (Running) in tree during execution
  - You can continue working while import runs
  - On completion: node flips to ✓ (UpToDate)
  - DuckDB uses parallel CSV parser via FIFO pipe

MCP SERVER (for AI agent integration)
  tplot exposes tools via Unix socket (auto-started):
    run_command       Execute any Tcl/SQL command
    list_nodes        Get lineage graph
    preview_table     Query table data (JSON)
    get_editor_content   Read cmd editor
    set_editor_content   Write to cmd editor

  --mcp-server flag: runs as stdio↔socket bridge (launched by kiro)

LINEAGE NODES
  [T]  Table — materialized data (from sql CREATE or into)
  [Q]  Query — derived view (from sql -name or derive)
  [P]  Plot  — visualization (from plot command)

STATUS ICONS
  ○  Empty (never run)
  ✓  Up to date
  ⚠  Dirty (upstream changed)
  >  Running (async in progress)
  ✗  Error (last run failed)

SESSION PERSISTENCE
  - Editor content saved to .tplot.state on exit
  - Node definitions saved to nodes/*.tcl on creation
  - Both restored on next launch

FILES
  .tplot/tplot.duckdb        DuckDB database
  .tplot.state               Session state (editor content)
  .tplot.log                 Application log
  nodes/*.tcl                Persisted node definitions
  .kiro/agents/tplot.json    Generated agent file for kiro
"
    .to_string()
}
