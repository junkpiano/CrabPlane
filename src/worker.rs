use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use crate::queue::{Queue, QueueError};
use crate::registry::Registry;
use crate::tasks::{TaskContext, TaskOutput};
use crate::types::Job;

#[derive(Debug)]
pub struct ResultItem {
    pub job: Job,
    pub output: TaskOutput,
    pub err: Option<String>,
    pub finished_at: SystemTime,
    pub dur: Duration,
}

pub struct Pool {
    q: Arc<Queue>,
    reg: Arc<Registry>,
    workers: usize,

    canceled: Arc<AtomicBool>,
    results_tx: Option<mpsc::Sender<ResultItem>>,
    joins: Vec<JoinHandle<()>>,
}

impl Pool {
    pub fn new(
        reg: Arc<Registry>,
        q: Arc<Queue>,
        workers: usize,
    ) -> (Self, mpsc::Receiver<ResultItem>) {
        let workers = if workers == 0 { 4 } else { workers };
        let (tx, rx) = mpsc::channel();
        (
            Self {
                q,
                reg,
                workers,
                canceled: Arc::new(AtomicBool::new(false)),
                results_tx: Some(tx),
                joins: Vec::new(),
            },
            rx,
        )
    }

    pub fn start(&mut self) {
        for worker_id in 1..=self.workers {
            let q = Arc::clone(&self.q);
            let reg = Arc::clone(&self.reg);
            let canceled = Arc::clone(&self.canceled);
            let tx = self.results_tx.as_ref().unwrap().clone();
            self.joins.push(thread::spawn(move || {
                run_worker(worker_id, q, reg, canceled, tx);
            }));
        }
    }

    pub fn submit(&self, job: Job) -> Result<(), String> {
        self.q
            .enqueue(job, &self.canceled)
            .map_err(|e| format!("failed to queue job: {e:?}"))
    }

    pub fn shutdown(&mut self) {
        self.q.close();
        self.canceled.store(true, Ordering::Relaxed);

        // Closing the sender ends the dispatch loop once workers exit.
        self.results_tx.take();

        for j in self.joins.drain(..) {
            let _ = j.join();
        }
    }
}

fn run_worker(
    worker_id: usize,
    q: Arc<Queue>,
    reg: Arc<Registry>,
    canceled: Arc<AtomicBool>,
    results_tx: mpsc::Sender<ResultItem>,
) {
    let ctx = TaskContext;
    loop {
        let job = match q.dequeue(&canceled) {
            Ok(j) => j,
            Err(QueueError::Closed | QueueError::Canceled) => return,
        };

        let start = Instant::now();
        let mut out = TaskOutput::None;
        let mut err: Option<String> = None;

        match reg.lookup(&job.task_name) {
            None => {
                err = Some(format!("unknown task: {}", job.task_name));
            }
            Some(task) => {
                if let Err(e) = task.validate(&job.input) {
                    err = Some(e);
                } else {
                    match task.run(&ctx, job.input.clone()) {
                        Ok(o) => out = o,
                        Err(e) => err = Some(e),
                    }
                }
            }
        }

        let finished_at = SystemTime::now();
        let dur = start.elapsed();

        let _ = results_tx.send(ResultItem {
            job,
            output: out,
            err,
            finished_at,
            dur,
        });

        // Avoid busy looping in case something goes wrong; tiny backoff is fine for v0.
        if canceled.load(Ordering::Relaxed) {
            return;
        }
        let _ = worker_id; // reserved for future structured logs
    }
}
