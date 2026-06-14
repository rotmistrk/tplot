# tplot — Terminal Data Analysis with Lineage Tracking

## What It Feels Like

```
tplot> import flows.csv
Importing... 2.3M rows [████████████████████] done (1.2s)
tplot> sql
  → opens SQL editor tab for current node
```

```sql
SELECT dst_ip, sum(bytes)/1e6 as mb, count(*) as flows
FROM flows WHERE src_port < 1024
GROUP BY dst_ip ORDER BY mb DESC LIMIT 20
```

Ctrl+Enter → result appears in Table tab. Edit script → creates variant branch.

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
- **Delete node** — removes data; scripts migrate to children for reconstructibility
- **Compare** — diff two variants

## Data Engine: DuckDB

Embedded columnar OLAP. Handles millions of rows, full SQL, native CSV/Parquet.
Nodes are DuckDB tables (materialized) or views (lazy, not stored).

Materialization is explicit: user chooses to materialize or keep as view.

## Three Script Languages

| Language | Purpose |
|----------|---------|
| SQL | Queries, transforms, aggregation |
| Tcl | Orchestration, multi-step pipelines, control flow |
| gnuplot | Chart rendering (subprocess → PNG) |

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
