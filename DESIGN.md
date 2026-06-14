# tplot — Terminal Data Analysis with Lineage Tracking

## What It Feels Like

```tcl
tplot> into auth [exec grep "failed" /var/log/auth.log] -regex {^(\S+).*user=(\S+).*from (\S+)} -cols {ts user ip}
Imported 4,231 rows → auth

tplot> plot bar [sql {SELECT user, count(*) as n FROM auth GROUP BY user ORDER BY n DESC LIMIT 10}]
[bar chart appears in Graph tab]

tplot> export [sql {SELECT * FROM auth WHERE user='root'}] -csv -name root_attempts.csv
Exported 892 rows → root_attempts.csv

tplot> derive by_hour {SELECT strftime(ts,'%H') as hour, count(*) as n FROM auth GROUP BY hour}
Created node: by_hour (child of auth)

tplot> freeze
Node auth frozen. Future edits will create a branch.
```

## Layout

```
┌───────────────┬─────────────────────────────────┬────────────┐
│ Left Panel    │ Center Panel                    │ Tools      │
│ (tabbed trees)│ (tabs for selected node)        │            │
│               │                                 │ ▸ Kiro     │
│[Lineage][Lib] │ [Script] [Table] [Graph] [Info] │   chat     │
│               │                                 │            │
│ ▸ Raw Flows   │  (active tab content)           │ ▸ Shell    │
│   ├─▸ TCP v1  │                                 │            │
│   │  ├─▸ ByDst│                                 │ ▸ Messages │
│   │  └─▸ Rate │                                 │            │
│   ├─▸ TCP v2  │                                 │            │
│   │  ├─◇ ByDst│                                 │            │
│   │  └─◇ Rate │                                 │            │
│   └─▸ HiPorts │                                 │            │
│                │                                 │            │
│ ▸ DNS Logs     │                                 │            │
├───────────────┴─────────────────────────────────┴────────────┤
│ Bottom Panel: command line + Kiro input                       │
├──────────────────────────────────────────────────────────────┤
│ [node: tcp-v1] [▸ running 2.1s] [2.3M rows]        tplot    │
└──────────────────────────────────────────────────────────────┘
```

The bottom panel is a full panel (expandable, can show multi-line command input or output).
The status bar is a single line below it — shows current node, progress, row counts, etc.

### Left Panel Tabs

| Tab | Content |
|-----|---------|
| Lineage | The analysis DAG tree (primary navigation) |
| Library | Reusable recipes tree (built-in + user + project) |
| Todo | Task tracking (analysis steps, findings, next actions) |

### Tools Panel Tabs (right)

| Tab | Content |
|-----|---------|
| Kiro | AI chat (MCP-connected, can run tplot commands) |
| Shell | PTY terminal (data collection scripts, file inspection) |
| Tcl | Interactive tplot REPL (primary analysis interface) |
| Messages | Log output, errors, progress |

### Bottom Panel

The bottom panel shows the status bar context. The Tcl REPL lives in the
Tools panel (right side), not the bottom — it needs multi-line input and
persistent visibility.

## The Lineage Tree

The left panel is the primary navigation. It represents the analysis DAG as a tree.

### Each node is:

- A **dataset** (DuckDB table/view)
- Produced by a **script** (SQL/Tcl/shell) that transforms its parent
- Has zero or more **views** (plots, stats summaries)
- Has a **registration card** (metadata)

### Node states:

| State | Icon | Meaning |
|-------|------|---------|
| Materialized | ▸ | Data exists, ready to query |
| Ghost | ◇ | Script inherited from variant, not yet run |
| Running | ⟳ | Computation in progress |
| Stale | ▸⚠ | Parent changed, data outdated |
| Error | ✗ | Script failed |

### DAG representation:

The lineage is a DAG (joins create multiple parents), rendered as a tree.
Multi-parent edges are shown as **link entries** — synthetic children that
point to alternate parents, similar to `..` in Unix but named and multi-parent:

```
▸ Raw Flows
  ├─▸ TCP Only
  │   └─▸ Joined with DNS
  │       ├── ⤴ DNS Logs         ← alt parent link (navigable)
  │       ├─▸ Top Talkers
  │       ...
▸ DNS Logs
  └── ⤴ → Joined with DNS       ← reverse link (who uses me)
```

Link entries are:
- Visually distinct (special glyph `⤴`, dimmed/italic color)
- Navigable — Enter jumps to the referenced node
- Show both directions: "I depend on X" (upward) and "X is used by Y" (reverse)
- Not real children — synthetic, rendered-only

## Center Panel: Node Views

When you select a tree node, the center panel shows tabs:

| Tab | Content |
|-----|---------|
| Script | The SQL/Tcl/gnuplot that produces this node from parent |
| Table | Query result (scrollable columns) |
| Graph | Plot output (one or more plot configs as sub-tabs/layers) |
| Info | Registration card (see below) |

### Registration Card (Info tab)

- Created timestamp
- Last run timestamp + duration
- Row count, column summary
- Execution log (stdout/stderr from the run)
- User comments / notes (stored in `doc.md`, editable in-place)
- Parent lineage (clickable path back to root)
- Variant-of reference (if this is a branch)
- Dependencies ("also uses: ...")

## Edit = Branch

Editing a node's script always creates a sibling variant:

```
Before:  A → B → C, D
After:   A → B  → C, D       (original preserved)
         A → B' → C', D'     (variant, children are ghosts)
```

Ghost children inherit scripts but have no data until explicitly run.
No data replication — ghosts cost nothing.

- **Edit** — creates variant (safe, non-destructive)
- **Edit in-place** — marks children stale (with confirmation)
- **Freeze** — seals node; any future edit auto-branches
- **Delete node** — removes entire subtree (data + scripts)
- **Trim data** — removes data/ but keeps scripts (nodes become Ghost)
- **Compare** — diff two variants

## Data Engine: DuckDB

Embedded columnar OLAP. Handles millions of rows, full SQL, native CSV/Parquet.
Nodes are DuckDB tables (materialized) or views (lazy, not stored).

Materialization is explicit: user chooses to materialize or keep as view.

## The tplot Language (Tcl)

Tcl is the scripting core of tplot. A node's script IS a `.tcl` file. SQL and gnuplot
are embedded sub-languages called from Tcl. External tools (grep, awk, python) produce
text that tplot structurizes into tables.

### Core Commands

| Command | Purpose |
|---------|---------|
| `into <table> <source> ?options?` | Import data into DuckDB table |
| `sql <query>` | Execute SQL, return result reference |
| `plot <type> <data> ?options?` | Render chart (gnuplot) |
| `gnuplot <script>` | Run raw gnuplot script block |
| `derive <name> <sql>` | Create child node from SQL transform |
| `export <result> -format -name <path>` | Export data/chart to file |
| `stats <table> ?options?` | Summary statistics |
| `hist <table> <col>` | Histogram |
| `cdf <table> <col>` | CDF/CCDF plot |
| `corr <table> <col1> <col2>` | Correlation |
| `node <name>` | Create/switch to node |
| `run` | Re-execute current node's script |
| `freeze` | Seal node (future edits auto-branch) |

### Structurization: `into` command

The `into` command bridges unstructured tool output → typed DuckDB columns:

```tcl
# Auto-detect format (CSV, TSV, JSON lines)
into flows [exec cat flows.csv]

# Explicit CSV
into data [exec curl -s http://api/data] -csv

# JSON lines
into events [exec journalctl -o json --since today] -json

# Regex parsing for unstructured logs
into auth [exec grep "failed" /var/log/auth.log] \
    -regex {^(\S+).*user=(\S+).*from (\S+)} \
    -cols {ts user ip}

# Space-separated with explicit headers
into vmstat [exec vmstat 1 10 | tail -n+3] -sep " " -header {r b swpd free buff cache}

# From file directly (DuckDB auto-detect)
into flows -file flows.csv
into events -file events.parquet
```

Format options: `-csv`, `-tsv`, `-json` (JSON lines), `-sep <char>`,
`-regex <pattern> -cols <names>`, `-file <path>`.

Compression options: `-gz`, `-zstd`, `-bz2` (auto-detected from extension
for `-file`, explicit for piped data). Decompression is streaming, constant memory.

Storage: DuckDB natively supports `s3://` paths via httpfs extension.

### Resource Budget

```tcl
budget -cpu 6          ;# DuckDB threads (default: total CPUs - 2)
budget -ram 4G         ;# DuckDB memory_limit (default: 75% available)
budget -disk 50G       ;# warn when cumulative node data exceeds this
```

### SQL integration

SQL is natively bound — results stay structured, no parsing needed:

```tcl
set result [sql {SELECT dst_ip, sum(bytes)/1e6 as mb FROM flows GROUP BY dst_ip ORDER BY mb DESC LIMIT 10}]
plot bar $result -x dst_ip -y mb
```

### Gnuplot integration

```tcl
gnuplot {
    set terminal pngcairo size 800,400
    set title "Traffic by Hour"
    set style data boxes
    plot $hourly using 1:2
}
```

### Export

```tcl
export [sql {SELECT * FROM flows}] -csv -name filtered.csv
export [sql {SELECT ip, cnt FROM top}] -jsonl -name top.jsonl
export [plot bar $result -x ip -y mb] -png -name chart.png
export [plot bar $result -x ip -y mb] -svg -name chart.svg
```

### Interactive Workflow

When a node is **active**, every command typed in the Tcl REPL is appended to
the node's script. The script IS the session history — fully reproducible.

```
User selects node → node becomes active
Each REPL command → appended to node's script.tcl
Script is always re-runnable (reproduces the node's data)
```

- `freeze` — seals the node. Any future edit auto-creates a branch.
- `run` — re-executes the script, regenerates data.
- Editing the script directly → marks node stale (needs re-run).

## Node Lifecycle

| State | Meaning |
|-------|---------|
| Active | User working here, commands logging to script |
| Frozen | Sealed. Edits auto-branch (clone subtree, ghost data) |
| Stale | Script edited, data doesn't match |
| Materialized | Data exists and matches current script |
| Ghost | Script inherited from branch, not yet run |
| Error | Last run failed |

### Metadata: Size and Timing

```toml
[size]
data_bytes = 45000000             # current node data
descendants_bytes = 120000000     # cumulative subtree
peak_bytes = 200000000            # historical max

[timing]
last_run_secs = 2.3
total_run_secs = 14.7             # cumulative all runs
run_count = 6
```

## Syntax Highlighting

The editor and REPL use **sub-language zone highlighting**:

- Tcl blocks highlighted with standard Tcl syntax coloring
- `sql {...}` blocks get a **blue-tinted background** + SQL keyword highlighting
- `gnuplot {...}` blocks get a **green-tinted background** + gnuplot syntax
- `-regex {...}` gets a **yellow-tinted background** + regex metachar highlighting
- `[exec ...]` gets a **gray-tinted background** + shell highlighting

The tinted background makes sub-language boundaries immediately visible without
matching braces mentally. Internal syntax highlighting applies within each zone.

## Completion

Readline-style completion in the REPL and editors:

| Context | Completes |
|---------|-----------|
| Command position | Tcl commands (`into`, `sql`, `plot`, ...) |
| After table name | Column names (from DuckDB schema) |
| Inside `sql {...}` | SQL keywords + table/column names |
| After `-file` | File paths |
| After `node`, `derive` | Existing node names |
| After `plot` | Chart types (`bar`, `scatter`, `hist`, ...) |
| Anywhere | History (fuzzy-matched, per-node + global) |

History is persistent (`.tplot/history`), per-node and global.

## Async Execution

Worker threads (std, no tokio). Progress to statusbar.
Long operations: import, large queries, multi-step Tcl pipelines.

## MCP Server

tplot exposes an MCP server (same pattern as kairn) so Kiro can:
- Create nodes, edit scripts, run computations
- Read table results, check node state
- Trigger and configure plots
- Monitor progress of async operations
- Navigate and query the lineage tree

### Tcl-defined MCP Tools

Library recipes can expose themselves as MCP tools via Tcl wrappers:

```tcl
# library/tools/top_talkers.tcl
mcp_tool top_talkers {table column n} {
    sql "SELECT $column, sum(bytes) as total FROM $table GROUP BY $column ORDER BY total DESC LIMIT $n"
}
```

Kiro can then call `top_talkers(table="flows", column="dst_ip", n=10)` directly.
No compilation, user-extensible.

### Library Contents

The library serves three audiences:

| Audience | What they see |
|----------|---------------|
| User | Browsable recipes, insertable snippets |
| Kiro | Skills and prompts (how to use tplot, available recipes) |
| MCP | Tool definitions (callable by Kiro or external agents) |

## Kiro Integration

The Kiro agent can:
- Generate SQL/Tcl/gnuplot from natural language descriptions
- Suggest library recipes based on data shape
- Explain query results and statistical output
- Recommend chart types for the current dataset
- Autonomously run multi-step analyses via MCP tools

## Library: Reusable Recipes

Parameterized snippets, browsable and insertable.

### Categories

- **Aggregation** — Top-N, pivot, group stats
- **Statistics** — percentiles, correlation, distribution fit, outlier detection
- **Distribution** — bimodal/multimodal partitioning, KDE, heavy-tail classification
- **Time-series** — rate computation, bucketing, moving average, periodicity (FFT)
- **Comparison** — A/B tests (t-test, KS, Mann-Whitney), change-point detection
- **Visualization** — CDF/CCDF, heatmap, QQ-plot, multi-axis
- **Cleaning** — dedup, gap detection, null handling, type coercion

### Storage

```
~/.tplot/library/       # user global (recipes, tools, skills)
project/library/        # project-local
built-in                # shipped defaults
```

Tools and MCP definitions are loaded from both `~/.tplot/library/` and the project's `library/` directory. Project-local overrides global.

## Project Disk Layout

A directory is a project. Scripts, docs, and metadata are version-controlled.
Data files are gitignored.

```
myproject/
  nodes/
    raw-flows/
      script.sql            # import/transform script
      meta.toml             # parent, dependencies, timestamps, state
      doc.md                # user notes, findings, comments
      views/
        top10.gp            # plot configs
        cdf.gp
        stats.tcl
      data/                 # ← GITIGNORED
        result.parquet
        cache.duckdb
    tcp-filtered/
      script.sql
      meta.toml
      doc.md
      views/
        ...
      data/                 # ← GITIGNORED
        ...
  library/                  # project-local recipes (version-controlled)
  scripts/                  # standalone scripts (data collection, etc.)
  docs/                     # project-level documentation
  .tplot/                   # UI state only
    settings.toml           # UI preferences
    state.json              # panel sizes, open tabs, cursor positions
  .gitignore                # ignores nodes/*/data/ and .tplot/
```

### What's git-tracked (reproducible):
- `nodes/*/script.sql` — all transform logic
- `nodes/*/meta.toml` — lineage, dependencies, comments
- `nodes/*/doc.md` — analysis notes
- `nodes/*/views/` — plot configs, stat scripts
- `library/` — project recipes
- `scripts/` — data collection scripts
- `docs/` — documentation

### What's gitignored (ephemeral/large):
- `nodes/*/data/` — materialized results (re-derivable from scripts)
- `.tplot/` — UI state (personal preference)

## Non-Goals

- Not a remote DB client
- Not a spreadsheet (no cell editing)
- Not a notebook (tabs, not interleaved cells)
- Not a BI dashboard (single-user local tool)

## Dependencies

- txv (TUI framework + inline images)
- duckdb-rs (embedded analytics DB)
- Tcl interpreter (from kairn infrastructure)
- gnuplot (subprocess, optional)
