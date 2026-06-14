---
name: estimate-task
description: Estimate LOE and split large tasks before implementation.
---

# Estimate Task Skill

## When to Use
Before executing any task from the todo tree.

## Procedure

1. **Read** the task title and note.

2. **Estimate LOE** (Fibonacci: 1, 2, 3, 5, 8, 13, 21):
   - 1 = trivial (one file, obvious change)
   - 2 = small (2-3 files, straightforward)
   - 3 = medium (multiple files, some thought needed)
   - 5 = large (cross-cutting, needs design)
   - 8+ = too big for one pass

3. **Set LOE** on the item via `set_loe`.

4. **If LOE > 3**: Split into subtasks (each ≤ 3) before starting.
   - First subtask: "Write failing scenario test"
   - Last subtask: "Verify all tests pass, commit"

5. **If unclear**: Add note with questions, move to next task.
