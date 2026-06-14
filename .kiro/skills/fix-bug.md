---
name: fix-bug
description: Bug fix cycle. Reproduces with test, fixes, verifies no regressions.
---

# Fix Bug Skill

## When to Use
When a bug is reported or discovered.

## Procedure

1. **Reproduce** — Write a scenario test that demonstrates the bug (MUST fail).

2. **Diagnose** — Read the relevant code path. Identify root cause.

3. **Fix** — Minimal change. After each file: `check_file`, fix violations.

4. **Verify** — `cargo fmt && cargo build && cargo test` (ALL tests).

5. **Commit** — Descriptive message referencing the bug symptoms.

## Anti-patterns
- Do NOT fix the test to match broken behavior
- Do NOT add `#[ignore]`
- If fix breaks other tests, diagnose deeper — the fix is wrong
