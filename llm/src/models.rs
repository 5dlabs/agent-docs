//! LLM models and types

use serde::{Deserialize, Serialize};

/// Supported LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProvider {
    /// Claude Code (local binary execution)
    ClaudeCode,
    /// OpenAI models
    OpenAI,
}

/// Model configuration for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The provider type
    pub provider: LlmProvider,
    /// Model identifier
    pub model_name: String,
    /// Maximum tokens for response
    pub max_tokens: u32,
    /// Temperature setting (0.0 to 1.0)
    pub temperature: f32,
    /// Path to binary (for local execution)
    pub binary_path: Option<String>,
    /// API key (for cloud providers)
    pub api_key: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::ClaudeCode,
            model_name: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1000,
            temperature: 0.7,
            binary_path: Some("claude".to_string()),
            api_key: None,
        }
    }
}

impl ModelConfig {
    /// Create a Claude Code configuration
    #[must_use]
    pub fn claude_code(binary_path: Option<String>) -> Self {
        Self {
            provider: LlmProvider::ClaudeCode,
            model_name: "claude-3-5-sonnet-20241022".to_string(),
            binary_path: binary_path.or_else(|| Some("claude".to_string())),
            ..Self::default()
        }
    }

    /// Create an OpenAI configuration
    #[must_use]
    pub fn openai(api_key: String) -> Self {
        Self {
            provider: LlmProvider::OpenAI,
            model_name: "gpt-4".to_string(),
            api_key: Some(api_key),
            binary_path: None,
            ..Self::default()
        }
    }
}

/// Message structure for LLM conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: String,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a user message
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a system message
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Response from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated text content
    pub content: String,
    /// Usage statistics (if available)
    pub usage: Option<Usage>,
    /// Model used for generation
    pub model: String,
}

/// Usage statistics for LLM calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens used
    pub prompt_tokens: Option<u32>,
    /// Output tokens generated
    pub completion_tokens: Option<u32>,
    /// Total tokens used
    pub total_tokens: Option<u32>,
}
