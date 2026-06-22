# tplot — Terminal Data Analysis with Lineage Tracking

A TUI application for interactive data analysis with full lineage tracking, async execution, and AI agent integration.

## Features

- **SQL queries** — DuckDB-powered, full SQL dialect including CTEs, window functions, PIVOT
- **Lineage graph** — every table, query, and plot tracked as a node with parent relationships
- **Async shell import** — `into table --shell {cmd} --csv` runs in background via FIFO, DuckDB parallel ingest
- **AI integration** — built-in MCP server, `M-x kiro` launches agent that can drive tplot programmatically
- **Vi-mode editor** — multi-line Tcl scripts with syntax highlighting and Ctrl-N completion
- **Session persistence** — editor content and node graph survive restarts

## Quick Start

```sh
cargo build --release
./target/release/tplot [project-dir]
```

Once running:
```
F4          → Focus command editor
i           → Insert mode
sql {CREATE TABLE demo AS SELECT 1 as x, 2 as y}
Esc, F9     → Execute
F2          → See lineage tree
```

## Commands

| Command | Description |
|---------|-------------|
| `sql {query}` | Execute SQL, show result |
| `sql -name tbl {query}` | Execute + register as named node |
| `into tbl -file path.csv` | Import file |
| `into tbl --shell {cmd} --csv` | Import from shell (async) |
| `plot bar tbl x y` | Bar chart |
| `derive name {sql}` | Derived query node |
| `shell` | Open PTY terminal |
| `kiro [--agent=name]` | Launch AI agent |

## Key Bindings

| Key | Action |
|-----|--------|
| F1 | Help |
| F2/F3/F4 | Focus tree/main/tools |
| F5 | Zoom panel |
| F9/F10 | Run line/all |
| Alt-x | Command line (M-x) |
| Ctrl-Q | Quit |
| Ctrl-N | Completion dropdown (in editor) |

### Lineage Tree
| Key | Action |
|-----|--------|
| Enter / → | Execute node |
| M-e | Copy command to editor |
| M-d | Delete subtree |
| M-c | Clone subtree |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ tplot TUI (txv framework)                               │
│  ┌──────────┐  ┌───────────────┐  ┌──────────────────┐ │
│  │ Lineage  │  │ Main (table/  │  │ Tools (editor/   │ │
│  │ Tree     │  │ plot views)   │  │ shell/kiro)      │ │
│  └──────────┘  └───────────────┘  └──────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Tcl scripting → Registry → DuckDB engine                │
│ Job manager (async) ← FIFO → parallel CSV ingest        │
├─────────────────────────────────────────────────────────┤
│ MCP server (Unix socket) ← kiro agent (via --mcp-server)│
└─────────────────────────────────────────────────────────┘
```

## MCP Tools (for AI agents)

When `M-x kiro` launches, the agent connects via MCP and can use:

- `run_command` — execute Tcl/SQL
- `list_nodes` — inspect lineage graph
- `preview_table` — query data
- `get_editor_content` / `set_editor_content` — read/write editor

## Dependencies

- [txv](https://github.com/rotmistrk/txv) — TUI framework
- [rusticle](https://github.com/rotmistrk/rusticle) — Tcl interpreter
- [DuckDB](https://duckdb.org/) — analytical SQL engine
- [syntect](https://github.com/trishume/syntect) — syntax highlighting

## License

MIT
