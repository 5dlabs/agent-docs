//! Embedding models and types

use serde::{Deserialize, Serialize};

/// Embedding request
#[derive(Debug, Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: String,
}

/// Embedding response (simplified for internal use)
#[derive(Debug)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
}

/// `OpenAI` API embedding response
#[derive(Debug, Deserialize)]
pub struct OpenAIEmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

/// Embedding data
#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}
