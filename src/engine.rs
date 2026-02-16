use std::sync::{Arc, RwLock, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::registry::Registry;
use crate::router::Router;
use crate::tasks::TaskOutput;
use crate::types::{Job, Message, Response};
use crate::worker::{Pool, ResultItem};

pub trait Engine: Send + Sync {
    fn handle(&self, msg: Message) -> Response;
}

pub trait ResultSink: Send + Sync {
    fn deliver(&self, job: &Job, resp: &Response) -> Result<(), String>;
}

pub struct Core {
    router: Arc<dyn Router>,
    reg: Arc<Registry>,
    pool: RwLock<Pool>,
    sink: RwLock<Option<Arc<dyn ResultSink>>>,
    dispatch_join: RwLock<Option<JoinHandle<()>>>,
}

impl Core {
    pub fn new(
        router: Arc<dyn Router>,
        reg: Arc<Registry>,
        mut pool: Pool,
        results_rx: mpsc::Receiver<ResultItem>,
        sink: Option<Arc<dyn ResultSink>>,
    ) -> Arc<Self> {
        pool.start();
        let c = Arc::new(Self {
            router,
            reg,
            pool: RwLock::new(pool),
            sink: RwLock::new(sink),
            dispatch_join: RwLock::new(None),
        });

        let c2 = Arc::clone(&c);
        let j = thread::spawn(move || c2.dispatch_results(results_rx));
        *c.dispatch_join.write().unwrap() = Some(j);
        c
    }

    pub fn set_sink(&self, s: Option<Arc<dyn ResultSink>>) {
        if let Ok(mut g) = self.sink.write() {
            *g = s;
        }
    }

    pub fn shutdown(&self) {
        if let Ok(mut p) = self.pool.write() {
            p.shutdown();
        }
        if let Ok(mut j) = self.dispatch_join.write() {
            if let Some(h) = j.take() {
                let _ = h.join();
            }
        }
    }

    fn dispatch_results(&self, results_rx: mpsc::Receiver<ResultItem>) {
        for res in results_rx {
            let text = format_result(&res);
            if text.is_empty() {
                continue;
            }
            let resp = Response {
                text,
                ephemeral: false,
            };

            let sink = self.sink.read().ok().and_then(|g| g.as_ref().cloned());
            let Some(sink) = sink else { continue };

            // v0: best-effort delivery, with a coarse timeout via a helper thread.
            let job = res.job;
            let resp2 = resp.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let r = sink.deliver(&job, &resp2);
                let _ = tx.send(r);
            });
            let _ = rx.recv_timeout(Duration::from_secs(10));
        }
    }
}

impl Engine for Core {
    fn handle(&self, msg: Message) -> Response {
        let route = match self.router.route(&msg) {
            Ok(None) => return Response::default(),
            Ok(Some(r)) => r,
            Err(e) => {
                return Response {
                    text: e,
                    ephemeral: true,
                };
            }
        };

        let task = match self.reg.lookup(&route.task_name) {
            Some(t) => t,
            None => {
                return Response {
                    text: format!("task not found: {}", route.task_name),
                    ephemeral: true,
                };
            }
        };

        if let Err(e) = task.validate(&route.input) {
            return Response {
                text: e,
                ephemeral: true,
            };
        }

        let job = Job {
            id: new_id(),
            task_name: route.task_name,
            input: route.input,
            user_id: msg.user_id,
            channel_id: msg.channel,
            created_at: SystemTime::now(),
        };

        if let Ok(p) = self.pool.read() {
            if let Err(e) = p.submit(job.clone()) {
                return Response {
                    text: e,
                    ephemeral: true,
                };
            }
        } else {
            return Response {
                text: "worker pool unavailable".to_string(),
                ephemeral: true,
            };
        }

        Response {
            text: queue_status_text(&job.task_name),
            ephemeral: true,
        }
    }
}

fn queue_status_text(task_name: &str) -> String {
    match task_name {
        // Ask-like tasks use adapter-level typing indicators where available.
        "ask" => String::new(),
        _ => "working...".to_string(),
    }
}

fn format_result(res: &ResultItem) -> String {
    if let Some(e) = &res.err {
        return format!("error: {}", e);
    }
    match &res.output {
        TaskOutput::None => "ok".to_string(),
        TaskOutput::Text(s) => s.to_string(),
    }
}

fn new_id() -> String {
    // 16 bytes hex-ish, using time + address entropy. Not cryptographic; good enough for v0.
    // Avoids external crates (uuid/rand/hex) due to offline build constraints.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let addr = (&now as *const u128 as usize) as u128;
    format!("{:032x}", now ^ addr)
}
