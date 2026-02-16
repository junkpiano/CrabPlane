use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use crate::types::Job;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    Closed,
    Canceled,
}

struct Inner {
    buf: VecDeque<Job>,
    closed: bool,
}

pub struct Queue {
    cap: usize,
    inner: Mutex<Inner>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl Queue {
    pub fn new(size: usize) -> Self {
        let cap = if size == 0 { 64 } else { size };
        Self {
            cap,
            inner: Mutex::new(Inner {
                buf: VecDeque::with_capacity(cap),
                closed: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    pub fn enqueue(
        &self,
        job: Job,
        canceled: &std::sync::atomic::AtomicBool,
    ) -> Result<(), QueueError> {
        let mut g = self.inner.lock().map_err(|_| QueueError::Closed)?;
        loop {
            if g.closed {
                return Err(QueueError::Closed);
            }
            if canceled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(QueueError::Canceled);
            }
            if g.buf.len() < self.cap {
                g.buf.push_back(job);
                self.not_empty.notify_one();
                return Ok(());
            }
            let (ng, _) = self
                .not_full
                .wait_timeout(g, Duration::from_millis(100))
                .map_err(|_| QueueError::Closed)?;
            g = ng;
        }
    }

    pub fn dequeue(&self, canceled: &std::sync::atomic::AtomicBool) -> Result<Job, QueueError> {
        let mut g = self.inner.lock().map_err(|_| QueueError::Closed)?;
        loop {
            if let Some(job) = g.buf.pop_front() {
                self.not_full.notify_one();
                return Ok(job);
            }
            if g.closed {
                return Err(QueueError::Closed);
            }
            if canceled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(QueueError::Canceled);
            }
            let (ng, _) = self
                .not_empty
                .wait_timeout(g, Duration::from_millis(100))
                .map_err(|_| QueueError::Closed)?;
            g = ng;
        }
    }

    pub fn close(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.closed = true;
            self.not_empty.notify_all();
            self.not_full.notify_all();
        }
    }
}
