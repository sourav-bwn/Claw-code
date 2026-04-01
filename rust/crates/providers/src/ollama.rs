use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use super::{Provider, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk, Usage, Message, Content, MessageRole, Choice};
use super::error::ProviderError;
use super::StreamingDelta;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self::with_base_url("http://localhost:11434")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: usize,
}

impl From<&Message> for OllamaMessage {
    fn from(msg: &Message) -> Self {
        let content = match &msg.content {
            Content::Text(t) => t.clone(),
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
        
        Self { role: role.to_string(), content }
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    message: OllamaResponseMessage,
    done: bool,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    prompt_eval_count: usize,
    #[serde(default)]
    eval_count: usize,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    model: String,
    message: OllamaResponseMessage,
    done: bool,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    prompt_eval_count: usize,
    #[serde(default)]
    eval_count: usize,
}

fn get_ollama_model(model: &str) -> String {
    let model = model.to_lowercase();
    if model.contains("ollama") || model.contains("local") {
        "llama3.2".to_string()
    } else {
        model.replace("ollama-", "")
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> {
        let ollama_messages: Vec<OllamaMessage> = request.messages.iter().map(OllamaMessage::from).collect();
        let model = get_ollama_model(&request.model);

        let ollama_request = OllamaRequest {
            model: model.clone(),
            messages: ollama_messages,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature.unwrap_or(0.8),
                num_predict: request.max_tokens.unwrap_or(4096),
            },
        };

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .header("content-type", "application/json")
            .json(&ollama_request)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(ProviderError::ModelNotFound(model));
        }

        if response.status() == 500 {
            return Err(ProviderError::Unknown("Ollama server error".to_string()));
        }

        let ollama_response: OllamaResponse = response.json().await?;

        Ok(ChatCompletionResponse {
            id: format!("ollama-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            model: ollama_response.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: Content::Text(ollama_response.message.content),
                },
                finish_reason: if ollama_response.done { Some("stop".to_string()) } else { None },
            }],
            usage: Usage {
                prompt_tokens: ollama_response.prompt_eval_count,
                completion_tokens: ollama_response.eval_count,
                total_tokens: ollama_response.prompt_eval_count + ollama_response.eval_count,
            },
            created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u64,
        })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<futures::stream::BoxStream<Result<StreamingChunk, ProviderError>>, ProviderError> {
        let ollama_messages: Vec<OllamaMessage> = request.messages.iter().map(OllamaMessage::from).collect();
        let model = get_ollama_model(&request.model);

        let ollama_request = OllamaRequest {
            model: model.clone(),
            messages: ollama_messages,
            stream: true,
            options: OllamaOptions {
                temperature: request.temperature.unwrap_or(0.8),
                num_predict: request.max_tokens.unwrap_or(4096),
            },
        };

        let response = self.client
            .post(format!("{}/api/chat", self.base_url))
            .header("content-type", "application/json")
            .json(&ollama_request)
            .send()
            .await?;

        if response.status() == 404 {
            return Err(ProviderError::ModelNotFound(model));
        }

        let stream = response.bytes_stream().map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if let Ok(event) = serde_json::from_str::<OllamaStreamResponse>(line) {
                            if !event.message.content.is_empty() {
                                return Ok(StreamingChunk {
                                    id: format!("ollama-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                                    delta: StreamingDelta::Content(event.message.content),
                                    usage: if event.done {
                                        Some(Usage {
                                            prompt_tokens: event.prompt_eval_count,
                                            completion_tokens: event.eval_count,
                                            total_tokens: event.prompt_eval_count + event.eval_count,
                                        })
                                    } else {
                                        None
                                    },
                                });
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
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct OllamaModelsResponse {
            models: Vec<OllamaModelInfo>,
        }
        #[derive(Deserialize)]
        struct OllamaModelInfo {
            name: String,
            #[serde(default)]
            model: Option<String>,
        }

        let models: OllamaModelsResponse = response.json().await?;

        Ok(models.models.into_iter().map(|m| super::Model {
            id: m.model.unwrap_or(m.name.clone()),
            name: m.name,
            provider: "ollama".to_string(),
        }).collect())
    }

    fn provider_name(&self) -> &str {
        "ollama"
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
