use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub id: String,
    pub model: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<String>,
    pub message: String,
    pub model: Option<String>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub message: String,
    pub model: String,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub model: String,
    pub created_at: String,
    pub message_count: usize,
}

pub struct SessionStore {
    sessions: Arc<RwLock<Vec<SessionData>>>,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub id: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: String,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create_session(&self, model: String) -> SessionData {
        let session = SessionData {
            id: uuid::Uuid::new_v4().to_string(),
            model,
            messages: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.sessions.write().await.push(session.clone());
        session
    }

    pub async fn get_session(&self, id: &str) -> Option<SessionData> {
        self.sessions.read().await
            .iter()
            .find(|s| s.id == id)
            .cloned()
    }

    pub async fn add_message(&self, session_id: &str, role: &str, content: &str) {
        if let Some(session) = self.sessions.write().await
            .iter_mut()
            .find(|s| s.id == session_id)
        {
            session.messages.push(ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: role.to_string(),
                content: content.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions.read().await
            .iter()
            .map(|s| SessionInfo {
                id: s.id.clone(),
                model: s.model.clone(),
                created_at: s.created_at.clone(),
                message_count: s.messages.len(),
            })
            .collect()
    }

    pub async fn delete_session(&self, id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        let len_before = sessions.len();
        sessions.retain(|s| s.id != id);
        sessions.len() < len_before
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
