use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub model: String,
    pub messages: Vec<StoredMessage>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, model: String) -> Session {
        let now = chrono::Utc::now().timestamp();
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            model,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: SessionMetadata::default(),
        };
        self.sessions.write().await.insert(session.id.clone(), session.clone());
        session
    }

    pub async fn get(&self, id: &str) -> Option<Session> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn update(&self, session: &Session) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.clone(), session.clone());
    }

    pub async fn delete(&self, id: &str) -> bool {
        self.sessions.write().await.remove(id).is_some()
    }

    pub async fn list(&self) -> Vec<Session> {
        self.sessions.read().await.values().cloned().collect()
    }

    pub async fn add_message(&self, session_id: &str, role: &str, content: &str) -> Option<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.messages.push(StoredMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: role.to_string(),
                content: content.to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            });
            session.updated_at = chrono::Utc::now().timestamp();
            Some(())
        } else {
            None
        }
    }

    pub async fn clear_messages(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.messages.clear();
            session.updated_at = chrono::Utc::now().timestamp();
            true
        } else {
            false
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
