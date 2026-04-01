use std::sync::Arc;
use std::collections::HashMap;

use super::{Provider, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk, ProviderType, get_provider_type, ProviderError};
use super::config::ProviderConfig;
use super::anthropic::AnthropicProvider;
use super::openai::OpenAIProvider;
use super::gemini::GeminiProvider;
use super::ollama::OllamaProvider;

pub struct ProviderRouter {
    providers: HashMap<ProviderType, Arc<dyn Provider>>,
    default_provider: ProviderType,
    default_model: String,
}

impl ProviderRouter {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let mut providers = HashMap::new();
        let mut default_provider = ProviderType::Anthropic;
        let mut default_model = "claude-sonnet-4-6".to_string();

        if let Some(ref anthropic_config) = config.anthropic {
            if let Some(ref api_key) = anthropic_config.api_key {
                if !api_key.is_empty() {
                    let provider: Arc<dyn Provider> = if let Some(ref base_url) = anthropic_config.base_url {
                        Arc::new(AnthropicProvider::with_base_url(api_key.clone(), base_url))
                    } else {
                        Arc::new(AnthropicProvider::new(api_key.clone()))
                    };
                    providers.insert(ProviderType::Anthropic, provider);
                    default_provider = ProviderType::Anthropic;
                    default_model = anthropic_config.default_model.clone();
                }
            }
        }

        if let Some(ref openai_config) = config.openai {
            if let Some(ref api_key) = openai_config.api_key {
                if !api_key.is_empty() {
                    let provider: Arc<dyn Provider> = if let Some(ref base_url) = openai_config.base_url {
                        Arc::new(OpenAIProvider::with_base_url(api_key.clone(), base_url))
                    } else {
                        Arc::new(OpenAIProvider::new(api_key.clone()))
                    };
                    providers.insert(ProviderType::OpenAI, provider);
                    if config.anthropic.is_none() {
                        default_provider = ProviderType::OpenAI;
                        default_model = openai_config.default_model.clone();
                    }
                }
            }
        }

        if let Some(ref gemini_config) = config.gemini {
            if let Some(ref api_key) = gemini_config.api_key {
                if !api_key.is_empty() {
                    let provider: Arc<dyn Provider> = Arc::new(GeminiProvider::new(api_key.clone()));
                    providers.insert(ProviderType::Gemini, provider);
                    if config.anthropic.is_none() && config.openai.is_none() {
                        default_provider = ProviderType::Gemini;
                        default_model = gemini_config.default_model.clone();
                    }
                }
            }
        }

        if let Some(ref ollama_config) = config.ollama {
            let provider: Arc<dyn Provider> = if let Some(ref base_url) = ollama_config.base_url {
                Arc::new(OllamaProvider::with_base_url(base_url))
            } else {
                Arc::new(OllamaProvider::new())
            };
            providers.insert(ProviderType::Ollama, provider);
            if config.anthropic.is_none() && config.openai.is_none() && config.gemini.is_none() {
                default_provider = ProviderType::Ollama;
                default_model = ollama_config.default_model.clone();
            }
        }

        if providers.is_empty() {
            return Err(ProviderError::ProviderNotConfigured(
                "No API providers configured. Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY, or USE_OLLAMA".to_string()
            ));
        }

        Ok(Self {
            providers,
            default_provider,
            default_model,
        })
    }

    pub fn get_provider(&self, model: &str) -> Result<(&Arc<dyn Provider>, ProviderType), ProviderError> {
        let provider_type = get_provider_type(model);
        
        if let Some(provider) = self.providers.get(&provider_type) {
            Ok((provider, provider_type))
        } else if let Some(default_provider) = self.providers.get(&self.default_provider) {
            Ok((default_provider, self.default_provider))
        } else {
            Err(ProviderError::ProviderNotConfigured(
                format!("Provider for model '{}' not configured", model)
            ))
        }
    }

    pub async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let request = ChatCompletionRequest {
            model: model.clone(),
            ..request
        };

        let (provider, _) = self.get_provider(&model)?;
        provider.chat_completion(request).await
    }

    pub async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<futures::stream::BoxStream<Result<StreamingChunk, ProviderError>>, ProviderError> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let request = ChatCompletionRequest {
            model: model.clone(),
            stream: true,
            ..request
        };

        let (provider, _) = self.get_provider(&model)?;
        provider.chat_completion_stream(request).await
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    pub fn available_providers(&self) -> Vec<ProviderType> {
        self.providers.keys().cloned().collect()
    }
}
