use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub anthropic: Option<AnthropicConfig>,
    #[serde(default)]
    pub openai: Option<OpenAIConfig>,
    #[serde(default)]
    pub gemini: Option<GeminiConfig>,
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
    #[serde(default)]
    pub custom: HashMap<String, CustomProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    #[serde(default = "default_model")]
    pub default_model: String,
}

fn default_model() -> String {
    "claude-sonnet-4-6".to_string()
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            default_model: default_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub organization: Option<String>,
    #[serde(default = "default_openai_model")]
    pub default_model: String,
}

fn default_openai_model() -> String {
    "gpt-4o".to_string()
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            organization: None,
            default_model: default_openai_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    #[serde(default = "default_gemini_model")]
    pub default_model: String,
}

fn default_gemini_model() -> String {
    "gemini-2.0-flash".to_string()
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            default_model: default_gemini_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: Option<String>,
    #[serde(default = "default_ollama_model")]
    pub default_model: String,
}

fn default_ollama_model() -> String {
    "llama3.2".to_string()
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            default_model: default_ollama_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub supports_streaming: bool,
}

impl ProviderConfig {
    pub fn from_env() -> Self {
        Self {
            anthropic: std::env::var("ANTHROPIC_API_KEY").ok().map(|key| AnthropicConfig {
                api_key: Some(key),
                base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
                default_model: std::env::var("ANTHROPIC_DEFAULT_MODEL").unwrap_or_else(|_| default_model()),
            }),
            openai: std::env::var("OPENAI_API_KEY").ok().map(|key| OpenAIConfig {
                api_key: Some(key),
                base_url: std::env::var("OPENAI_BASE_URL").ok(),
                organization: std::env::var("OPENAI_ORG_ID").ok(),
                default_model: std::env::var("OPENAI_DEFAULT_MODEL").unwrap_or_else(|_| default_openai_model()),
            }),
            gemini: std::env::var("GEMINI_API_KEY").ok().map(|key| GeminiConfig {
                api_key: Some(key),
                default_model: std::env::var("GEMINI_DEFAULT_MODEL").unwrap_or_else(|_| default_gemini_model()),
            }),
            ollama: if std::env::var("OLLAMA_BASE_URL").is_ok() || std::env::var("USE_OLLAMA").is_ok() {
                Some(OllamaConfig {
                    base_url: std::env::var("OLLAMA_BASE_URL").ok(),
                    default_model: std::env::var("OLLAMA_DEFAULT_MODEL").unwrap_or_else(|_| default_ollama_model()),
                })
            } else {
                None
            },
            custom: HashMap::new(),
        }
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let contents = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, contents)
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
