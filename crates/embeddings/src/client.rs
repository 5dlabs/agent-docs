//! OpenAI embedding client

use crate::models::{EmbeddingRequest, EmbeddingResponse};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::{debug, error};

/// Trait for embedding clients
#[async_trait]
pub trait EmbeddingClient {
    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embedding using the client's API
    async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;
}

/// OpenAI embedding client implementation
pub struct OpenAIEmbeddingClient {
    client: Client,
    api_key: String,
}

impl OpenAIEmbeddingClient {
    /// Create a new embedding client
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy-key".to_string()); // Allow dummy key for testing

        let client = Client::new();

        Ok(Self { client, api_key })
    }
}

#[async_trait]
impl EmbeddingClient for OpenAIEmbeddingClient {
    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: text.to_string(),
            model: "text-embedding-3-large".to_string(),
        };

        let response = self.generate_embedding(request).await?;
        Ok(response.embedding)
    }

    /// Generate embedding using OpenAI API
    async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!(
            "Generating embedding for {} characters",
            request.input.len()
        );

        let payload = json!({
            "input": request.input,
            "model": request.model,
            "encoding_format": "float"
        });

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI API error: {}", error_text);
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let api_response: serde_json::Value = response.json().await?;

        let embedding = api_response
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| anyhow!("Invalid response format from OpenAI API"))?;

        let embedding_vec: Result<Vec<f32>, _> = embedding
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(|f| f as f32)
                    .ok_or_else(|| anyhow!("Invalid embedding value"))
            })
            .collect();

        let embedding_vec = embedding_vec?;

        debug!(
            "Generated embedding with {} dimensions",
            embedding_vec.len()
        );

        Ok(EmbeddingResponse {
            embedding: embedding_vec,
        })
    }
}
