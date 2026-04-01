pub mod error;
pub mod traits;
pub mod anthropic;
pub mod openai;
pub mod gemini;
pub mod ollama;
pub mod config;
pub mod router;

pub use error::ProviderError;
pub use traits::{Provider, Model, Message, MessageRole, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk};
pub use config::ProviderConfig;
pub use router::ProviderRouter;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub max_tokens: usize,
    pub supports_streaming: bool,
    pub supports_vision: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Gemini,
    Ollama,
    Custom,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Gemini => write!(f, "gemini"),
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::Custom => write!(f, "custom"),
        }
    }
}

pub fn get_available_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-opus-4-6".to_string(),
            provider: "anthropic".to_string(),
            display_name: "Claude Opus 4".to_string(),
            max_tokens: 200_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "claude-sonnet-4-6".to_string(),
            provider: "anthropic".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            max_tokens: 200_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "claude-haiku-4-5-20251213".to_string(),
            provider: "anthropic".to_string(),
            display_name: "Claude Haiku 4".to_string(),
            max_tokens: 200_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-4o".to_string(),
            max_tokens: 128_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-4o Mini".to_string(),
            max_tokens: 128_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gpt-4-turbo".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-4 Turbo".to_string(),
            max_tokens: 128_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gpt-3.5-turbo".to_string(),
            provider: "openai".to_string(),
            display_name: "GPT-3.5 Turbo".to_string(),
            max_tokens: 16_385,
            supports_streaming: true,
            supports_vision: false,
        },
        ModelInfo {
            id: "gemini-2.0-flash".to_string(),
            provider: "gemini".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
            max_tokens: 1_000_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gemini-2.0-flash-exp".to_string(),
            provider: "gemini".to_string(),
            display_name: "Gemini 2.0 Flash Experimental".to_string(),
            max_tokens: 1_000_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            provider: "gemini".to_string(),
            display_name: "Gemini 1.5 Pro".to_string(),
            max_tokens: 2_000_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            provider: "gemini".to_string(),
            display_name: "Gemini 1.5 Flash".to_string(),
            max_tokens: 1_000_000,
            supports_streaming: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "ollama-local".to_string(),
            provider: "ollama".to_string(),
            display_name: "Ollama (Local)".to_string(),
            max_tokens: 8192,
            supports_streaming: true,
            supports_vision: false,
        },
    ]
}

pub fn resolve_model_alias(alias: &str) -> String {
    match alias.to_lowercase().as_str() {
        "opus" => "claude-opus-4-6".to_string(),
        "sonnet" => "claude-sonnet-4-6".to_string(),
        "haiku" => "claude-haiku-4-5-20251213".to_string(),
        "gpt4" | "gpt-4" => "gpt-4o".to_string(),
        "gpt4-mini" | "gpt-4-mini" => "gpt-4o-mini".to_string(),
        "gemini" | "gemini-flash" => "gemini-2.0-flash".to_string(),
        "gemini-pro" => "gemini-1.5-pro".to_string(),
        "local" | "ollama" => "ollama-local".to_string(),
        other => other.to_string(),
    }
}

pub fn get_provider_type(model_id: &str) -> ProviderType {
    let model = model_id.to_lowercase();
    if model.contains("claude") {
        ProviderType::Anthropic
    } else if model.contains("gpt") || model.contains("openai") {
        ProviderType::OpenAI
    } else if model.contains("gemini") {
        ProviderType::Gemini
    } else if model.contains("ollama") || model.contains("llama") || model.contains("mistral") {
        ProviderType::Ollama
    } else {
        ProviderType::Custom
    }
}
