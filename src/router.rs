use crate::types::{Message, TaskInput};

#[derive(Clone, Debug)]
pub struct Route {
    pub task_name: String,
    pub input: TaskInput,
}

pub trait Router: Send + Sync {
    // Ok(None): not a command
    // Ok(Some(Route)): valid command -> route
    // Err(String): command-like input but invalid/unknown
    fn route(&self, msg: &Message) -> Result<Option<Route>, String>;
}

// PrefixRouter implements v0 prefix-based routing:
// - !ping
// - !echo <text>
// - !ask <prompt>
// - any other non-empty message -> default ask task (selected backend)
#[derive(Clone, Debug, Default)]
pub struct PrefixRouter;

impl PrefixRouter {
    pub fn new() -> Self {
        Self
    }
}

impl Router for PrefixRouter {
    fn route(&self, msg: &Message) -> Result<Option<Route>, String> {
        let text = msg.text.trim();
        if text.is_empty() {
            return Ok(None);
        }

        if text == "!ping" {
            return Ok(Some(Route {
                task_name: "ping".to_string(),
                input: TaskInput::Empty,
            }));
        }

        if let Some(rest) = text.strip_prefix("!echo") {
            let rest = rest.trim();
            if rest.is_empty() {
                return Err("usage: !echo <text>".to_string());
            }
            return Ok(Some(Route {
                task_name: "echo".to_string(),
                input: TaskInput::Text(rest.to_string()),
            }));
        }

        if let Some(rest) = text.strip_prefix("!ask") {
            let rest = rest.trim();
            if rest.is_empty() {
                return Err("usage: !ask <prompt>".to_string());
            }
            return Ok(Some(Route {
                task_name: "ask".to_string(),
                input: TaskInput::Text(rest.to_string()),
            }));
        }

        Ok(Some(Route {
            task_name: "ask".to_string(),
            input: TaskInput::Text(text.to_string()),
        }))
    }
}
