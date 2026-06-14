---
name: deferred-implementation
description: Design traits and interfaces for features implemented later, without cutting corners on the API.
---

# Deferred Implementation Skill

## When to Use
When a feature is architecturally important but not the current priority.
We need the interface settled now to avoid redesign, but implementation can wait.

## Procedure

1. **Define the Trait** — Full method signatures, doc comments, correct self types.

2. **Default Impls** — Provide defaults that:
   - Log "not yet implemented: <method_name>" at warn level
   - Return sensible zero-values (empty vec, None, Ok(()))
   - Never panic

3. **Compile-Check Integration** — The trait must be callable from its real callsite:
   - Instantiate with the dummy impl
   - Call from the handler/view/wherever it will eventually be used
   - Verify no borrow conflicts at the callsite

4. **Document Expectations** — In the trait doc comment, describe:
   - What a real implementation must guarantee
   - Thread safety requirements
   - Performance expectations (called per-frame? per-command?)

5. **Todo Entry** — Add to the todo tree with:
   - Reference to the trait file/line
   - Note describing what "done" looks like
   - LOE estimate

## Anti-patterns
- Don't skip the trait and "figure it out later"
- Don't use `todo!()` or `unimplemented!()` (these panic)
- Don't define traits with methods you're unsure about — better to add methods later than remove them
- Don't create overly generic traits (trait per concrete use case is fine)
