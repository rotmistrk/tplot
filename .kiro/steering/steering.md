# tplot — Agent Steering Document

## Implementation Workflow

Every feature follows: Estimate → Split → Test → Implement → Lint → Verify.

1. Pick story from todo tree
2. Estimate LOE (fibonacci). If > 3, split into subtasks.
3. Write scenario test that tells the user story (MUST fail initially)
4. Implement minimal code to pass the test
5. After modifying EACH file: run `check_file`, fix violations immediately
6. Cover edge cases with unit tests
7. Run full build + test cycle
8. Mark done

## Code Standards

- `rustfmt.toml`: `max_width = 120`
- No `unwrap()`/`expect()`/`panic!()` in runtime code
- 240 code lines max per file (blank/comment lines don't count)
- 40 code lines max per function
- 7 parameters max per function
- No bare `pub` fields — use `pub(crate)` + accessors

## Architecture

- Built on txv framework (same as kairn)
- DuckDB for data engine (embedded, columnar OLAP)
- Tcl for orchestration (embedded interpreter)
- gnuplot for chart rendering (subprocess)
- std threads + channels for async (no tokio)
- MCP server for Kiro integration

## Project Layout

```
src/              Application code
tests/            Scenario tests (one concern per file)
library/          Built-in recipes, tools, skills
nodes/            Data lineage tree (scripts git-tracked, data/ gitignored)
.kiro/            Agent config (steering, skills)
.tplot/           UI state (gitignored)
```

## Consulting Kairn

The kairn project at `../kairn/f4` is the reference implementation for:
- txv framework usage (TiledWorkspace, panels, tabs, views)
- MCP server pattern
- Tcl scripting integration
- Test harness patterns (`tests/helpers.rs`)
- Handler dispatch pattern
- Command/event architecture

Consult kairn code when implementing equivalent features.

## Test Strategy

- Scenario tests in `tests/`: demonstrate user stories against TestHarness
- Unit tests inline: cover edge cases, parsing, data model logic
- All tests deterministic, no shared state, parallelizable
- Never `#[ignore]` — fix or delete

## No Silent Errors

Every failure must be visible: statusbar message, log at WARN/ERROR, or error propagation.
Never `let _ = fallible_op();` without logging.

## Dependencies

- Use exact/pinned versions where possible
- Prefer well-known, maintained crates
- Check kairn's Cargo.toml for patterns before adding new deps
