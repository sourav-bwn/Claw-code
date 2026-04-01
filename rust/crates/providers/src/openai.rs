use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use super::{Provider, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk, Usage, Message, Content, MessageRole, Choice};
use super::error::ProviderError;
use super::StreamingDelta;

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    organization: Option<String>,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com".to_string(),
            organization: None,
        }
    }

    pub fn with_base_url(api_key: String, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.into(),
            organization: None,
        }
    }

    pub fn with_organization(api_key: String, org: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com".to_string(),
            organization: Some(org.into()),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl From<&Message> for OpenAIMessage {
    fn from(msg: &Message) -> Self {
        let content = match &msg.content {
            Content::Text(text) => text.clone(),
            Content::Parts(parts) => parts
                .iter()
                .filter_map(|p| {
                    if let super::ContentPart::Text { text } = p {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };
        
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        
        Self { role: role.to_string(), content, name: None }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
    created: u64,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: usize,
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    id: String,
    object: String,
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: usize,
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    role: Option<String>,
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> {
        let openai_messages: Vec<OpenAIMessage> = request.messages.iter().map(OpenAIMessage::from).collect();
        
        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages: openai_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stream: false,
            stop: None,
        };

        let mut req_builder = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json");

        if let Some(ref org) = self.organization {
            req_builder = req_builder.header("OpenAI-Organization", org);
        }

        let response = req_builder.json(&openai_request).send().await?;

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        if response.status() == 429 {
            return Err(ProviderError::RateLimitExceeded);
        }

        let openai_response: OpenAIResponse = response.json().await?;

        let message = Message {
            role: MessageRole::Assistant,
            content: Content::Text(openai_response.choices[0].message.content.clone()),
        };

        let usage = Usage {
            prompt_tokens: openai_response.usage.prompt_tokens,
            completion_tokens: openai_response.usage.completion_tokens,
            total_tokens: openai_response.usage.total_tokens,
        };

        Ok(ChatCompletionResponse {
            id: openai_response.id,
            model: openai_response.model,
            choices: vec![Choice {
                index: 0,
                message,
                finish_reason: openai_response.choices[0].finish_reason.clone(),
            }],
            usage,
            created: openai_response.created,
        })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<futures::stream::BoxStream<Result<StreamingChunk, ProviderError>>, ProviderError> {
        let openai_messages: Vec<OpenAIMessage> = request.messages.iter().map(OpenAIMessage::from).collect();
        
        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages: openai_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stream: true,
            stop: None,
        };

        let mut req_builder = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json");

        if let Some(ref org) = self.organization {
            req_builder = req_builder.header("OpenAI-Organization", org);
        }

        let response = req_builder.json(&openai_request).send().await?;

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        let stream = response.bytes_stream().map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(event) = serde_json::from_str::<OpenAIStreamResponse>(data) {
                                let delta_text = event.choices[0].delta.content.clone().unwrap_or_default();
                                
                                if !delta_text.is_empty() || event.choices[0].finish_reason.is_some() {
                                    return Ok(StreamingChunk {
                                        id: event.id.clone(),
                                        delta: StreamingDelta::Content(delta_text),
                                        usage: event.usage.map(|u| Usage {
                                            prompt_tokens: u.prompt_tokens,
                                            completion_tokens: u.completion_tokens,
                                            total_tokens: u.total_tokens,
                                        }),
                                    });
                                }
                            }
                        }
                    }
                    Ok(StreamingChunk {
                        id: "pending".to_string(),
                        delta: StreamingDelta::Content(String::new()),
                        usage: None,
                    })
                }
                Err(e) => Err(ProviderError::StreamingError(e.to_string())),
            }
        }).filter(|chunk| {
            futures::future::ready(match chunk {
                Ok(c) => !c.delta_is_empty(),
                Err(_) => true,
            })
        });

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<super::Model>, ProviderError> {
        let response = self.client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelData>,
        }
        #[derive(Deserialize)]
        struct ModelData {
            id: String,
            #[serde(default)]
            name: Option<String>,
        }

        let models: ModelsResponse = response.json().await?;

        Ok(models.data.into_iter().map(|m| super::Model {
            id: m.id,
            name: m.name.unwrap_or_else(|| m.id.clone()),
            provider: "openai".to_string(),
        }).collect())
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

impl StreamingChunk {
    pub fn delta_is_empty(&self) -> bool {
        match &self.delta {
            StreamingDelta::Content(s) => s.is_empty(),
            StreamingDelta::Parts(p) => p.is_empty(),
        }
    }
}
