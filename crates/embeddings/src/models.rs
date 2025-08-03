//! Embedding models and types

use serde::{Deserialize, Serialize};

/// Embedding request
#[derive(Debug, Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: String,
}

/// Embedding response
#[derive(Debug, Deserialize)]
pub struct EmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

/// Embedding data
#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}