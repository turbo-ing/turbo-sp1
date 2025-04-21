use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProveStatus {
    InProgress,
    Done(serde_json::Value),
    Error(String),
}

pub struct ProveQueue {
    tasks: Arc<Mutex<HashMap<String, ProveStatus>>>,
    handles: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
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
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn enqueue_task(&self) -> String {
        let id = Uuid::new_v4().to_string();
        self.tasks
            .lock()
            .unwrap()
            .insert(id.clone(), ProveStatus::InProgress);
        id
    }

    pub fn get_status(&self, id: &str) -> Option<ProveStatus> {
        self.tasks.lock().unwrap().get(id).cloned()
    }

    pub fn set_status(&self, id: &String, status: ProveStatus) {
        self.tasks.lock().unwrap().insert(id.to_string(), status);
    }

    pub fn store_handle(&self, id: &String, handle: JoinHandle<()>) {
        self.handles.lock().unwrap().insert(id.to_string(), handle);
    }

    pub fn cleanup_handle(&self, id: &str) {
        self.handles.lock().unwrap().remove(id);
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
