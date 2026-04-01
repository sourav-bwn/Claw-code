use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("API request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Streaming error: {0}")]
    StreamingError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Timeout error")]
    Timeout,
    
    #[error("Unknown provider error: {0}")]
    Unknown(String),
}

impl ProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::RateLimitExceeded | ProviderError::Timeout
        )
    }
}

impl serde::Serialize for ProviderError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
