use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct Message {
    pub user_id: String,
    pub channel: String,
    pub text: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub text: String,
    pub ephemeral: bool,
}

#[derive(Clone, Debug)]
pub enum TaskInput {
    Empty,
    Text(String),
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub task_name: String,
    pub input: TaskInput,
    pub user_id: String,
    pub channel_id: String,
    pub created_at: SystemTime,
}
