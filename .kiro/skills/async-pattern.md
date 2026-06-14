---
name: async-pattern
description: Standard pattern for all background/long-running operations in tplot.
---

# Async Work Pattern

## When to Use
Any operation that may take more than ~100ms: imports, large queries,
gnuplot rendering, S3 transfers, exec pipes.

## The Pattern

Every async operation follows this structure:

```rust
// 1. Cancellation token
let cancel = Arc::new(AtomicBool::new(false));

// 2. Progress channel
let (tx, rx) = mpsc::channel::<Progress>();

// 3. Spawn worker
let cancel_clone = cancel.clone();
thread::spawn(move || {
    tx.send(Progress::Started { task: "..." }).ok();
    
    for chunk in work_items {
        if cancel_clone.load(Ordering::Relaxed) {
            tx.send(Progress::Cancelled).ok();
            return;
        }
        // do work...
        tx.send(Progress::Update { done, total }).ok();
    }
    
    tx.send(Progress::Done { result }).ok();
});

// 4. Store handle
job_manager.register(JobHandle { rx, cancel, node_id, started_at });
```

## Progress Enum

```rust
enum Progress {
    Started { task: String },
    Update { rows_done: u64, rows_total: Option<u64>, bytes_done: u64 },
    Done { result: Result<QueryResult, String> },
    Cancelled,
}
```

## UI Polling

On every `Tick` event (16ms), the handler:
1. Iterates all active job handles
2. Drains their rx channels (non-blocking: `rx.try_recv()`)
3. Updates node state + tree-table cells
4. Removes completed/cancelled jobs

## Cancel

User presses cancel key on a running node:
1. Handler looks up the job by node_id
2. Sets `cancel.store(true, Ordering::Relaxed)`
3. Worker detects it on next iteration, sends Cancelled, exits
4. Node state → Error or back to Stale

## ETA Calculation

```rust
let elapsed = started_at.elapsed();
let rate = rows_done as f64 / elapsed.as_secs_f64();
let remaining = (rows_total - rows_done) as f64 / rate;
```

## Rules

- Every job MUST check cancel periodically (at least once per chunk/batch)
- Every job MUST send Done or Cancelled before thread exits
- Progress channel is bounded (capacity 64) — worker must not block on send
- Never hold a lock while doing I/O
- JobManager lives in AppState, not in any View
