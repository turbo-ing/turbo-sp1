use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProveStatus {
    Queued,
    InProgress,
    Done(serde_json::Value),
    Error(String),
}

pub struct ProveQueue {
    tasks: Arc<Mutex<HashMap<String, ProveStatus>>>,
}

impl Default for ProveQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ProveQueue {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn enqueue_task(&self) -> String {
        let id = Uuid::new_v4().to_string();
        self.tasks
            .lock()
            .unwrap()
            .insert(id.clone(), ProveStatus::Queued);
        id
    }

    pub fn get_status(&self, id: &str) -> Option<ProveStatus> {
        self.tasks.lock().unwrap().get(id).cloned()
    }

    pub fn set_status(&self, id: &String, status: ProveStatus) {
        self.tasks.lock().unwrap().insert(id.to_string(), status);
    }
}

#[derive(Clone)]
pub struct ProveQueueHandle(Arc<ProveQueue>);

impl Default for ProveQueueHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ProveQueueHandle {
    pub fn new() -> Self {
        Self(Arc::new(ProveQueue::new()))
    }

    pub fn inner(&self) -> Arc<ProveQueue> {
        self.0.clone()
    }
}
