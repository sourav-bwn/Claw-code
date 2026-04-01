use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use super::{Provider, ChatCompletionRequest, ChatCompletionResponse, StreamingChunk, Usage, Message, Content, MessageRole, Choice};
use super::error::ProviderError;
use super::StreamingDelta;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    max_output_tokens: usize,
    temperature: f32,
    top_p: f32,
}

impl From<&Message> for GeminiContent {
    fn from(msg: &Message) -> Self {
        let role = match msg.role {
            MessageRole::System | MessageRole::User => "user",
            MessageRole::Assistant => "model",
        };
        
        let text = match &msg.content {
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
        
        Self {
            role: role.to_string(),
            parts: vec![GeminiPart { text }],
        }
    }
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiCandidateContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    prompt_token_count: usize,
    candidates_token_count: usize,
    total_token_count: usize,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamResponse {
    #[serde(rename = "candidates")]
    candidates: Option<Vec<GeminiStreamCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamCandidate {
    #[serde(rename = "content")]
    content: Option<GeminiCandidateContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

fn get_gemini_model_name(model: &str) -> String {
    if model.contains("gemini-2.0-flash-exp") {
        return "gemini-2.0-flash-exp".to_string();
    }
    if model.contains("gemini-2.0-flash") {
        return "gemini-2.0-flash".to_string();
    }
    if model.contains("gemini-1.5-pro") {
        return "gemini-1.5-pro".to_string();
    }
    if model.contains("gemini-1.5-flash") {
        return "gemini-1.5-flash".to_string();
    }
    model.to_string()
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse, ProviderError> {
        let contents: Vec<GeminiContent> = request.messages.iter().map(GeminiContent::from).collect();
        
        let system_instruction = request.system.as_ref().map(|s| GeminiSystemInstruction {
            parts: vec![GeminiPart { text: s.clone() }],
        });

        let gemini_request = GeminiRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: request.max_tokens.unwrap_or(8192),
                temperature: request.temperature.unwrap_or(0.9),
                top_p: request.top_p.unwrap_or(0.95),
            },
        };

        let model = get_gemini_model_name(&request.model);

        let response = self.client
            .post(format!(
                "{}/v1beta/models/{}:generateContent?key={}",
                self.base_url, model, self.api_key
            ))
            .header("content-type", "application/json")
            .json(&gemini_request)
            .send()
            .await?;

        if response.status() == 400 {
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(text));
        }

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let candidates = gemini_response.candidates.ok_or_else(|| {
            ProviderError::Unknown("No candidates in response".to_string())
        })?;

        let text = candidates[0].content.parts
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let usage = gemini_response.usage_metadata.map(|u| Usage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
        }).unwrap_or(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(ChatCompletionResponse {
            id: format!("gemini-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            model: request.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: MessageRole::Assistant,
                    content: Content::Text(text),
                },
                finish_reason: candidates[0].finish_reason.clone(),
            }],
            usage,
            created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as u64,
        })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<futures::stream::BoxStream<Result<StreamingChunk, ProviderError>>, ProviderError> {
        let contents: Vec<GeminiContent> = request.messages.iter().map(GeminiContent::from).collect();
        
        let system_instruction = request.system.as_ref().map(|s| GeminiSystemInstruction {
            parts: vec![GeminiPart { text: s.clone() }],
        });

        let gemini_request = GeminiRequest {
            contents,
            system_instruction,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: request.max_tokens.unwrap_or(8192),
                temperature: request.temperature.unwrap_or(0.9),
                top_p: request.top_p.unwrap_or(0.95),
            },
        };

        let model = get_gemini_model_name(&request.model);
        let model_id = request.model;

        let response = self.client
            .post(format!(
                "{}/v1beta/models/{}:streamGenerateContent?key={}",
                self.base_url, model, self.api_key
            ))
            .header("content-type", "application/json")
            .json(&gemini_request)
            .send()
            .await?;

        if response.status() == 401 {
            return Err(ProviderError::InvalidApiKey);
        }

        let stream = response.bytes_stream().map(move |result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    if let Ok(event) = serde_json::from_str::<GeminiStreamResponse>(&text) {
                        if let Some(candidates) = event.candidates {
                            if let Some(candidate) = candidates.first() {
                                if let Some(content) = &candidate.content {
                                    let delta_text = content.parts.iter()
                                        .map(|p| p.text.clone())
                                        .collect::<Vec<_>>()
                                        .join("");

                                    if !delta_text.is_empty() {
                                        return Ok(StreamingChunk {
                                            id: format!("gemini-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
                                            delta: StreamingDelta::Content(delta_text),
                                            usage: event.usage_metadata.map(|u| Usage {
                                                prompt_tokens: u.prompt_token_count,
                                                completion_tokens: u.candidates_token_count,
                                                total_tokens: u.total_token_count,
                                            }),
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
                id: "gemini-2.0-flash".to_string(),
                name: "Gemini 2.0 Flash".to_string(),
                provider: "gemini".to_string(),
            },
            super::Model {
                id: "gemini-2.0-flash-exp".to_string(),
                name: "Gemini 2.0 Flash Experimental".to_string(),
                provider: "gemini".to_string(),
            },
            super::Model {
                id: "gemini-1.5-pro".to_string(),
                name: "Gemini 1.5 Pro".to_string(),
                provider: "gemini".to_string(),
            },
            super::Model {
                id: "gemini-1.5-flash".to_string(),
                name: "Gemini 1.5 Flash".to_string(),
                provider: "gemini".to_string(),
            },
        ])
    }

    fn provider_name(&self) -> &str {
        "gemini"
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
