use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::conversation::{Conversation, Message};
use super::tools::ToolResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub conversation: Conversation,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: SessionMetadata,
    pub usage: UsageStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub system_prompt: Option<String>,
    pub working_directory: Option<String>,
    pub project_name: Option<String>,
    pub context_files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_tokens: usize,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_cost: f64,
    pub request_count: usize,
}

impl Session {
    pub fn new(model: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            conversation: Conversation::new(),
            model: model.into(),
            created_at: now,
            updated_at: now,
            metadata: SessionMetadata {
                system_prompt: None,
                working_directory: std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
                project_name: None,
                context_files: Vec::new(),
            },
            usage: UsageStats::default(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.metadata.system_prompt = Some(prompt.into());
        self
    }

    pub fn add_message(&mut self, role: MessageRole, content: impl Into<Content>) {
        self.conversation.add_message(role, content);
        self.updated_at = Utc::now();
    }

    pub fn add_user_message(&mut self, content: impl Into<Content>) {
        self.add_message(MessageRole::User, content);
    }

    pub fn add_assistant_message(&mut self, content: impl Into<Content>) {
        self.add_message(MessageRole::Assistant, content);
    }

    pub fn update_usage(&mut self, prompt_tokens: usize, completion_tokens: usize, cost: f64) {
        self.usage.prompt_tokens += prompt_tokens;
        self.usage.completion_tokens += completion_tokens;
        self.usage.total_tokens += prompt_tokens + completion_tokens;
        self.usage.total_cost += cost;
        self.usage.request_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn compact_history(&mut self, keep_last_n: usize) {
        self.conversation.compact(keep_last_n);
        self.updated_at = Utc::now();
    }

    pub fn clear(&mut self) {
        self.conversation.clear();
        self.updated_at = Utc::now();
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
        self.updated_at = Utc::now();
    }
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
    active_session: Option<String>,
    storage_path: Option<std::path::PathBuf>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_session: None,
            storage_path: None,
        }
    }

    pub fn with_storage_path(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.storage_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn create_session(&mut self, model: impl Into<String>) -> &mut Session {
        let session = Session::new(model);
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.active_session = Some(id);
        self.sessions.get_mut(&self.active_session.clone().unwrap()).unwrap()
    }

    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    pub fn get_active_session(&self) -> Option<&Session> {
        self.active_session.as_ref().and_then(|id| self.sessions.get(id))
    }

    pub fn get_active_session_mut(&mut self) -> Option<&mut Session> {
        self.active_session.as_ref().and_then(|id| self.sessions.get_mut(id))
    }

    pub fn set_active_session(&mut self, id: &str) -> bool {
        if self.sessions.contains_key(id) {
            self.active_session = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn delete_session(&mut self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }

    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(ref path) = self.storage_path {
            let json = serde_json::to_string_pretty(&self.sessions)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let json = std::fs::read_to_string(path)?;
                self.sessions = serde_json::from_str(&json)?;
            }
        }
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
