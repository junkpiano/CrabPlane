use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::engine::{Engine, ResultSink};
use crate::types::{Job, Message, Response};

pub struct Adapter {
    eng: Arc<dyn Engine>,
    out: Mutex<Box<dyn Write + Send>>,
}

impl Adapter {
    pub fn new(eng: Arc<dyn Engine>) -> Self {
        Self {
            eng,
            out: Mutex::new(Box::new(io::stdout())),
        }
    }

    pub fn run(&self, stop: &std::sync::atomic::AtomicBool) -> io::Result<()> {
        {
            let mut out = self.out.lock().unwrap();
            writeln!(
                out,
                "Crabplane CLI. Try: !ping, !echo hello, or !ask <prompt>"
            )?;
        }

        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let stdin = io::stdin();
            for line in stdin.lock().lines().flatten() {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if tx.send(line).is_err() {
                    return;
                }
            }
        });

        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    let resp = self.eng.handle(Message {
                        user_id: "cli".to_string(),
                        channel: "cli".to_string(),
                        text: line,
                        metadata: HashMap::new(),
                    });
                    if !resp.text.is_empty() {
                        let mut out = self.out.lock().unwrap();
                        writeln!(out, "{}", resp.text)?;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }
}

pub struct Sink {
    out: Mutex<Box<dyn Write + Send>>,
}

impl Sink {
    pub fn new() -> Self {
        Self {
            out: Mutex::new(Box::new(io::stdout())),
        }
    }
}

impl ResultSink for Sink {
    fn deliver(&self, _job: &Job, resp: &Response) -> Result<(), String> {
        if resp.text.is_empty() {
            return Ok(());
        }
        let mut out = self
            .out
            .lock()
            .map_err(|_| "stdout lock poisoned".to_string())?;
        writeln!(out, "{}", resp.text).map_err(|e| e.to_string())
    }
}
