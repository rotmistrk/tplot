//! Job manager — tracks background operations with progress and cancellation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

/// Progress update from a background job.
#[derive(Debug, Clone)]
pub(crate) enum Progress {
    Started {
        task: String,
    },
    Update {
        rows_done: u64,
        rows_total: Option<u64>,
        bytes_done: u64,
    },
    Done {
        result: Result<String, String>,
    },
    Cancelled,
}

/// Token to signal cancellation to a worker thread.
pub(crate) type CancelToken = Arc<AtomicBool>;

/// Create a new cancel token (initially false).
pub(crate) fn new_cancel_token() -> CancelToken {
    Arc::new(AtomicBool::new(false))
}

/// Handle to a running job.
pub(crate) struct JobHandle {
    pub(crate) node_id: String,
    pub(crate) task: String,
    pub(crate) rx: mpsc::Receiver<Progress>,
    pub(crate) cancel: CancelToken,
    pub(crate) started_at: Instant,
    // Latest progress snapshot.
    pub(crate) rows_done: u64,
    pub(crate) rows_total: Option<u64>,
    pub(crate) bytes_done: u64,
}

impl JobHandle {
    /// Request cancellation.
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Estimated time remaining in seconds, if total is known.
    pub(crate) fn eta_secs(&self) -> Option<f64> {
        let total = self.rows_total?;
        if self.rows_done == 0 {
            return None;
        }
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let rate = self.rows_done as f64 / elapsed;
        let remaining = (total - self.rows_done) as f64 / rate;
        Some(remaining)
    }

    /// Elapsed time in seconds.
    pub(crate) fn elapsed_secs(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }
}

/// Manages all active background jobs.
pub(crate) struct JobManager {
    jobs: Vec<JobHandle>,
}

impl JobManager {
    pub(crate) fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Register a new job.
    pub(crate) fn register(&mut self, handle: JobHandle) {
        self.jobs.push(handle);
    }

    /// Poll all jobs for progress updates. Returns node_ids that completed.
    pub(crate) fn poll(&mut self) -> Vec<(String, Result<String, String>)> {
        let mut completed = Vec::new();

        for job in &mut self.jobs {
            while let Ok(msg) = job.rx.try_recv() {
                match msg {
                    Progress::Started { .. } => {}
                    Progress::Update {
                        rows_done,
                        rows_total,
                        bytes_done,
                    } => {
                        job.rows_done = rows_done;
                        job.rows_total = rows_total;
                        job.bytes_done = bytes_done;
                    }
                    Progress::Done { result } => {
                        completed.push((job.node_id.clone(), result));
                    }
                    Progress::Cancelled => {
                        completed.push((job.node_id.clone(), Err("cancelled".to_string())));
                    }
                }
            }
        }

        // Remove completed jobs.
        let done_ids: Vec<String> = completed.iter().map(|(id, _)| id.clone()).collect();
        self.jobs.retain(|j| !done_ids.contains(&j.node_id));

        completed
    }

    /// Cancel a job by node_id.
    pub(crate) fn cancel_node(&self, node_id: &str) {
        if let Some(job) = self.jobs.iter().find(|j| j.node_id == node_id) {
            job.cancel();
        }
    }

    /// Get active job count.
    pub(crate) fn active_count(&self) -> usize {
        self.jobs.len()
    }

    /// Get job for a specific node (for status display).
    pub(crate) fn job_for_node(&self, node_id: &str) -> Option<&JobHandle> {
        self.jobs.iter().find(|j| j.node_id == node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_job_lifecycle() {
        let mut mgr = JobManager::new();
        let (tx, rx) = mpsc::channel();
        let cancel = new_cancel_token();

        mgr.register(JobHandle {
            node_id: "1.0".to_string(),
            task: "import".to_string(),
            rx,
            cancel: cancel.clone(),
            started_at: Instant::now(),
            rows_done: 0,
            rows_total: None,
            bytes_done: 0,
        });

        assert_eq!(mgr.active_count(), 1);

        // Simulate progress.
        tx.send(Progress::Update {
            rows_done: 500,
            rows_total: Some(1000),
            bytes_done: 5000,
        })
        .unwrap();
        tx.send(Progress::Done {
            result: Ok("done".to_string()),
        })
        .unwrap();

        let completed = mgr.poll();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].0, "1.0");
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn test_cancel() {
        let mut mgr = JobManager::new();
        let (tx, rx) = mpsc::channel();
        let cancel = new_cancel_token();

        let cancel_clone = cancel.clone();
        mgr.register(JobHandle {
            node_id: "test".to_string(),
            task: "long op".to_string(),
            rx,
            cancel,
            started_at: Instant::now(),
            rows_done: 0,
            rows_total: None,
            bytes_done: 0,
        });

        // Spawn worker that respects cancel.
        thread::spawn(move || {
            for i in 0..100 {
                if cancel_clone.load(Ordering::Relaxed) {
                    tx.send(Progress::Cancelled).unwrap();
                    return;
                }
                tx.send(Progress::Update {
                    rows_done: i,
                    rows_total: Some(100),
                    bytes_done: 0,
                })
                .unwrap();
                thread::sleep(std::time::Duration::from_millis(1));
            }
            tx.send(Progress::Done {
                result: Ok("done".to_string()),
            })
            .unwrap();
        });

        // Cancel immediately.
        mgr.cancel_node("test");
        thread::sleep(std::time::Duration::from_millis(50));

        let completed = mgr.poll();
        assert_eq!(completed.len(), 1);
        assert!(completed[0].1.is_err());
    }
}
