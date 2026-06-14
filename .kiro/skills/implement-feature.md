---
name: implement-feature
description: Test-first implementation cycle for tplot. Writes failing test, implements, lints every file, builds, runs tests.
---

# Implement Feature Skill

## When to Use
When implementing a new feature or fixing a bug in tplot.

## Procedure

1. **Estimate** — Set LOE on the task. If LOE > 3, split into subtasks first.

2. **Test First** — Write a scenario test in `tests/` that demonstrates the desired behavior. The test MUST fail initially.

3. **Implement** — Write minimal code to make the test pass.
   - After modifying EACH file: run `check_file` on it, fix violations immediately
   - Max 240 code lines per file, 40 per function, 7 params

4. **Update Help** — Update `src/help.rs` to reflect new/changed commands:
   - Mark newly implemented commands with ✓
   - Add working examples that use the new feature
   - Keep planned commands marked with ○

5. **Verify** — Run the full cycle:
   - `cargo fmt`
   - `cargo build`
   - `cargo clippy -- -D warnings`
   - `cargo test`

5. **Verify** — Run the full cycle:
   - `cargo fmt`
   - `cargo build`
   - `cargo clippy -- -D warnings`
   - `cargo test`

6. **Commit** — Stage, commit with descriptive message.

## Consulting Kairn
When implementing a feature that has a kairn equivalent:
1. Find the relevant kairn code at `../kairn/f4/src/`
2. Study the pattern (handler, view, command dispatch)
3. Adapt to tplot's domain (don't copy verbatim)

## Context Budget
- If context is running low: commit current progress, mark partial, stop
- Prefer small focused changes over large refactors
