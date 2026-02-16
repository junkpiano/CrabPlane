use std::sync::Arc;

use crate::engine::{Engine, ResultSink};
use crate::types::{Job, Response};

pub struct Adapter {
    token: String,
    _eng: Arc<dyn Engine>,
}

impl Adapter {
    pub fn new(token: String, eng: Arc<dyn Engine>) -> Self {
        Self { token, _eng: eng }
    }

    pub fn run(&self) -> Result<(), String> {
        if self.token.is_empty() {
            return Err("DISCORD_TOKEN is empty".to_string());
        }
        Err("discord adapter not implemented in this offline/no-deps Rust port".to_string())
    }

    pub fn close(&self) -> Result<(), String> {
        Ok(())
    }
}

impl ResultSink for Adapter {
    fn deliver(&self, _job: &Job, _resp: &Response) -> Result<(), String> {
        // No-op in the stub implementation.
        Ok(())
    }
}
