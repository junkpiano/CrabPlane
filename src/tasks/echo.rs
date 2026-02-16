use crate::tasks::{Task, TaskContext, TaskOutput};
use crate::types::TaskInput;

#[derive(Default)]
pub struct EchoTask;

impl EchoTask {
    pub fn new() -> Self {
        Self
    }
}

impl Task for EchoTask {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn validate(&self, input: &TaskInput) -> Result<(), String> {
        match input {
            TaskInput::Text(t) if !t.is_empty() => Ok(()),
            TaskInput::Text(_) => Err("text is empty".to_string()),
            _ => Err("invalid input".to_string()),
        }
    }

    fn run(&self, _ctx: &TaskContext, input: TaskInput) -> Result<TaskOutput, String> {
        match input {
            TaskInput::Text(t) => Ok(TaskOutput::Text(t)),
            _ => Err("invalid input".to_string()),
        }
    }
}
