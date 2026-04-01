use std::sync::Arc;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "message")]
    Message { content: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "clear")]
    Clear,
    #[serde(rename = "model")]
    Model { name: String },
}

#[derive(Debug, Clone)]
pub struct WebSocketRoom {
    sessions: Arc<std::sync::Mutex<Vec<broadcast::Sender<String>>>>,
}

impl WebSocketRoom {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        let (tx, rx) = broadcast::channel(100);
        self.sessions.lock().unwrap().push(tx);
        rx
    }

    pub fn broadcast(&self, message: &str) {
        let sessions = self.sessions.lock().unwrap();
        for session in sessions.iter() {
            let _ = session.send(message.to_string());
        }
    }

    pub fn client_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
}

impl Default for WebSocketRoom {
    fn default() -> Self {
        Self::new()
    }
}
