//! LLM client implementation

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

/// Claude API response structure
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

/// Claude API request structure
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// LLM client for Claude integration
pub struct LlmClient {
    client: Client,
    api_key: String,
}

impl LlmClient {
    /// Create a new LLM client
    ///
    /// # Errors
    ///
    /// Returns an error if the ANTHROPIC_API_KEY environment variable is not set.
    pub fn new() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Summarize text using Claude
    ///
    /// # Errors
    ///
    /// Returns an error if the Claude API call fails or returns an invalid response.
    pub async fn summarize(&self, text: &str) -> Result<String> {
        if text.len() > 50000 { // Claude has token limits
            return Ok(format!("Text too long ({} chars), truncated analysis", text.len()));
        }

        let request = ClaudeRequest {
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: 1000,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: format!("Please analyze this text and provide a concise summary:\n\n{}", text),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Claude API error: {}", error_text));
        }

        let claude_response: ClaudeResponse = response.json().await?;
        let summary = claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "No summary available".to_string());

        Ok(summary)
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new().expect("Failed to create LLM client - check ANTHROPIC_API_KEY")
    }
}
