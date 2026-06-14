---
name: architecture-decision
description: Use before implementing features that touch ownership, lifetimes, or cross-component communication.
---

# Architecture Decision Skill

## When to Use
Before implementing any feature that:
- Introduces shared mutable state
- Crosses thread boundaries
- Requires new traits that multiple components implement
- Changes how components communicate

## Procedure

1. **Draw Ownership** — Who owns the data? Who borrows? What lifetimes?
   - Owner = the struct that holds it as a field
   - Borrower = takes &self or &mut self reference
   - Thread boundary = requires Send + Sync or channel

2. **Identify Conflicts** — Can two things need &mut to the same data simultaneously?
   - Handler dispatching commands to multiple subsystems
   - UI drawing while background work mutates state
   - If yes: channel-based communication or split the struct

3. **Choose Pattern**:
   - Same thread, sequential access → &mut passed through handler
   - Cross-thread, write-once → Arc<data> (immutable sharing)
   - Cross-thread, progress/results → mpsc::channel
   - Cross-thread, cancellation → Arc<AtomicBool>
   - Never: Arc<Mutex<T>> for things the UI reads frequently (causes jank)

4. **Write Trait First** — Define the interface:
   - Method signatures with correct `&self`/`&mut self`
   - Associated types for results
   - Compile-check: can the callsite use it without borrow conflicts?

5. **Implement** — Only after steps 1-4 pass review.

## Anti-patterns
- Don't wrap everything in Arc<Mutex<>> to "make it work"
- Don't use RefCell in multi-threaded contexts
- Don't pass &mut AppState into background threads
- Don't store references in structs (use owned data or indices)
