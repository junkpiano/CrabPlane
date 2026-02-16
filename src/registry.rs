use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::tasks::Task;

#[derive(Default)]
pub struct Registry {
    tasks: RwLock<HashMap<String, Arc<dyn Task>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, t: Arc<dyn Task>) -> Result<(), String> {
        let name = t.name();
        if name.is_empty() {
            return Err("registry: task name is empty".to_string());
        }

        let mut g = self
            .tasks
            .write()
            .map_err(|_| "registry: poisoned lock".to_string())?;
        if g.contains_key(name) {
            return Err(format!("registry: task already registered: {name}"));
        }
        g.insert(name.to_string(), t);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<Arc<dyn Task>> {
        let g = self.tasks.read().ok()?;
        g.get(name).cloned()
    }
}
