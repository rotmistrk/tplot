//! Help text — comprehensive command reference with examples.
//! Updated on each coding iteration. ✓ = implemented, ○ = planned.

pub(crate) fn help_text() -> String {
    "\
═══ tplot — Terminal Data Analysis ═══

QUICK START
  # Generate sample data (no CSV needed):
  sql {CREATE TABLE auth AS SELECT * FROM (VALUES ('2024-01-01 10:00:01','root','192.168.1.100','failed'), ('2024-01-01 10:00:03','admin','10.0.0.5','failed'), ('2024-01-01 10:01:15','root','192.168.1.100','failed'), ('2024-01-01 10:02:30','deploy','172.16.0.1','failed'), ('2024-01-01 10:05:00','root','192.168.1.100','failed')) AS t(ts, username, src_ip, status)}

  # Query it:
  sql -name by_user {SELECT username, count(*) as attempts FROM auth GROUP BY username ORDER BY attempts DESC}
  sql -name by_ip {SELECT src_ip, count(*) as attempts FROM auth GROUP BY src_ip ORDER BY attempts DESC}

  # Or import a file:
  into mytable -file /path/to/data.csv

COMMANDS
  ✓ into <table> <source> ?opts?       Import data into DuckDB
  ✓ sql <query>                        Execute SQL, return result
  ✓ derive <name> <sql>                Create child node from query
  ✓ freeze                             Seal node (edits auto-branch)
  ✓ run                                Re-execute current node script
  ○ plot <type> <data> ?opts?          Render chart (gnuplot)
  ○ export <result> -fmt -name <path>  Export data/chart to file
  ○ stats <table> ?opts?               Summary statistics
  ○ hist <table> <col>                 Histogram
  ○ cdf <table> <col>                  CDF/CCDF plot
  ○ corr <table> <col1> <col2>        Correlation
  ○ budget -cpu N -ram XG -disk XG     Set resource limits
  ○ node <id>                          Switch to node

IMPORT FORMAT OPTIONS
  -file <path>    Read from file (auto-detects csv/tsv/parquet/json)
  -csv            Force CSV parsing
  -tsv            Force TSV parsing
  -json           JSON lines format
  -sep <char>     Custom separator
  -regex <pat> -cols {a b c}   Parse with regex capture groups
  -gz -zstd -bz2              Decompression (auto-detected for -file)

EXPORT FORMATS (planned)
  -csv -jsonl -parquet         Data formats
  -png -svg                    Chart formats
  -name <path>                 Output file path

NODE IDS
  0, 1, 2...                   Root nodes (auto-numbered)
  1.0, 1.1                     Children (dotted path)
  1.base                       Named reference (via names.toml)
  1.base.0                     Grandchild of named node

EXAMPLES
  # Import CSV and query
  into flows -file /tmp/netflows.csv
  sql {SELECT dst_ip, sum(bytes) as total FROM flows GROUP BY dst_ip ORDER BY total DESC LIMIT 10}

  # Derive a filtered subset
  derive tcp_only {SELECT * FROM flows WHERE protocol = 'tcp'}

  # Import from command output (planned)
  into auth [exec grep 'failed' /var/log/auth.log] -regex {^(\\S+).*user=(\\S+)} -cols {ts user}

KEYS
  F1          This help
  F2          Focus lineage tree
  Ctrl+Q      Quit
  Ctrl+C      Cancel running operation (on focused node)

STATUS ICONS (in lineage tree)
  ✓  Materialized (data ready)
  ▸  Active (user working here)
  ❄  Frozen (sealed, edits branch)
  ◇  Ghost (script only, no data)
  ⚠  Stale (needs re-run)
  >  Running (in progress)
  ✗  Error (last run failed)
"
    .to_string()
}
