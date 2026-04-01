use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Content {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Text(s) => write!(f, "{}", s),
            Content::Parts(parts) => {
                for part in parts {
                    match part {
                        ContentPart::Text { text } => write!(f, "{}", text)?,
                        ContentPart::Image { .. } => write!(f, "[Image]")?,
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    Text { text: String },
    Image { source: ImageSource },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResultInfo>,
}

impl Message {
    pub fn new(role: MessageRole, content: impl Into<Content>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.into(),
            tool_calls: None,
            tool_result: None,
        }
    }

    pub fn system(content: impl Into<Content>) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn user(content: impl Into<Content>) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: impl Into<Content>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(calls);
        self
    }

    pub fn with_tool_result(mut self, result: ToolResultInfo) -> Self {
        self.tool_result = Some(result);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Message>,
    pub max_messages: Option<usize>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_messages: None,
        }
    }

    pub fn with_max_messages(mut self, max: usize) -> Self {
        self.max_messages = Some(max);
        self
    }

    pub fn add_message(&mut self, role: MessageRole, content: impl Into<Content>) {
        self.messages.push(Message::new(role, content));
    }

    pub fn add_message_with_id(&mut self, id: String, role: MessageRole, content: impl Into<Content>) {
        let mut msg = Message::new(role, content);
        msg.id = id;
        self.messages.push(msg);
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn get_last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn compact(&mut self, keep_last_n: usize) {
        if keep_last_n > 0 {
            let system_messages: Vec<Message> = self.messages
                .iter()
                .filter(|m| m.role == MessageRole::System)
                .cloned()
                .collect();

            let user_messages: Vec<Message> = self.messages
                .iter()
                .filter(|m| m.role == MessageRole::User)
                .rev()
                .take(keep_last_n)
                .cloned()
                .collect();

            let assistant_messages: Vec<Message> = self.messages
                .iter()
                .filter(|m| m.role == MessageRole::Assistant)
                .cloned()
                .collect();

            let mut new_messages = system_messages;
            for user_msg in user_messages.iter().rev() {
                new_messages.push(user_msg.clone());
                if let Some(assistant) = assistant_messages.iter().find(|a| {
                    if let Some(idx) = new_messages.iter().position(|m| m.id == a.id) {
                        idx >= new_messages.len() - 2
                    } else {
                        false
                    }
                }) {
                    new_messages.push(assistant.clone());
                }
            }

            self.messages = new_messages;
        }
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}
