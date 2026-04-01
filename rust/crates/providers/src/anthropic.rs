use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use futures::StreamExt;

use super::{Provider, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk, Usage, Message, Content, MessageRole, Choice};
use super::error::ProviderError;
use super::StreamingDelta;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    pub fn with_base_url(api_key: String, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
    stream: bool,
    system: Option<String>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

impl From<&Message> for AnthropicMessage {
    fn from(msg: &Message) -> Self {
        let content = match &msg.content {
            Content::Text(text) => text.clone(),
            Content::Parts(parts) => parts
                .iter()
                .map(|p| serde_json::to_string(p).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n"),
        };
        
        let role = match msg.role {
            MessageRole::System => "user",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        };
        
        Self { role: role.to_string(), content }
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    index: Option<usize>,
    content_block: Option<AnthropicContentBlock>,
    delta: Option<AnthropicDelta>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> {
        let anthropic_messages: Vec<AnthropicMessage> = request.messages.iter().map(AnthropicMessage::from).collect();
        
        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages: anthropic_messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            stream: false,
            system: request.system,
            temperature: request.temperature,
        };

        let response = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await?;

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        let anthropic_response: AnthropicResponse = response.json().await?;

        let text = anthropic_response.content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let usage = Usage {
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            total_tokens: anthropic_response.usage.input_tokens + anthropic_response.usage.output_tokens,
        };

        Ok(ChatCompletionResponse {
            id: anthropic_response.id,
            model: anthropic_response.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: Content::Text(text),
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage,
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u64,
        })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<futures::stream::BoxStream<Result<StreamingChunk, ProviderError>>, ProviderError> {
        let anthropic_messages: Vec<AnthropicMessage> = request.messages.iter().map(AnthropicMessage::from).collect();
        
        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages: anthropic_messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            stream: true,
            system: request.system,
            temperature: request.temperature,
        };

        let response = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&anthropic_request)
            .send()
            .await?;

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        let stream = response.bytes_stream();
        let model = request.model;
        
        let stream = stream.map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                continue;
                            }
                            if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                if event.event_type == "content_block_delta" {
                                    let delta_text = event.delta
                                        .and_then(|d| d.text)
                                        .unwrap_or_default();
                                    
                                    if !delta_text.is_empty() {
                                        return Ok(StreamingChunk {
                                            id: format!("anthropic-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                                            delta: StreamingDelta::Content(delta_text),
                                            usage: None,
                                        });
                                    }
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
        Ok(vec![
            super::Model {
                id: "claude-opus-4-6".to_string(),
                name: "Claude Opus 4".to_string(),
                provider: "anthropic".to_string(),
            },
            super::Model {
                id: "claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider: "anthropic".to_string(),
            },
            super::Model {
                id: "claude-haiku-4-5-20251213".to_string(),
                name: "Claude Haiku 4".to_string(),
                provider: "anthropic".to_string(),
            },
        ])
    }

    fn provider_name(&self) -> &str {
        "anthropic"
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
