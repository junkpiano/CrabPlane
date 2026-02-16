use crate::tasks::{Task, TaskContext, TaskOutput};
use crate::types::TaskInput;

#[derive(Default)]
pub struct PingTask;

impl PingTask {
    pub fn new() -> Self {
        Self
    }
}

impl Task for PingTask {
    fn name(&self) -> &'static str {
        "ping"
    }

    fn validate(&self, _input: &TaskInput) -> Result<(), String> {
        Ok(())
    }

    fn run(&self, _ctx: &TaskContext, _input: TaskInput) -> Result<TaskOutput, String> {
        Ok(TaskOutput::Text("pong".to_string()))
    }
}
