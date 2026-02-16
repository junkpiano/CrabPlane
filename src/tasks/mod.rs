mod echo;
mod openai;
mod ping;

use crate::types::TaskInput;

pub use echo::EchoTask;
pub use openai::OpenAiTask;
pub use ping::PingTask;

#[derive(Clone, Debug)]
pub enum TaskOutput {
    None,
    Text(String),
}

pub struct TaskContext;

pub trait Task: Send + Sync {
    fn name(&self) -> &'static str;
    fn validate(&self, input: &TaskInput) -> Result<(), String>;
    fn run(&self, ctx: &TaskContext, input: TaskInput) -> Result<TaskOutput, String>;
}
