//! Embedding models and types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The cost per 1M tokens for text-embedding-3-large (batch API has 50% discount)
const EMBEDDING_COST_PER_MILLION_TOKENS: f64 = 0.13;
const BATCH_DISCOUNT_FACTOR: f64 = 0.5;

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
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: Option<usize>,
}

// === OpenAI Batch API Models ===

/// `OpenAI` Batch API request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub input_file_id: String,
    pub endpoint: String,
    pub completion_window: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// `OpenAI` Batch API response structure
#[derive(Debug, Clone, Deserialize)]
pub struct BatchResponse {
    pub id: String,
    pub object: String,
    pub endpoint: String,
    pub errors: Option<BatchErrors>,
    pub input_file_id: String,
    pub completion_window: String,
    pub status: BatchStatus,
    pub output_file_id: Option<String>,
    pub error_file_id: Option<String>,
    pub created_at: i64,
    pub in_progress_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub finalizing_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub failed_at: Option<i64>,
    pub expired_at: Option<i64>,
    pub cancelling_at: Option<i64>,
    pub cancelled_at: Option<i64>,
    pub request_counts: Option<BatchRequestCounts>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Batch status enumeration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Validating,
    Failed,
    InProgress,
    Finalizing,
    Completed,
    Expired,
    Cancelling,
    Cancelled,
}

/// Batch errors information
#[derive(Debug, Clone, Deserialize)]
pub struct BatchErrors {
    pub object: String,
    pub data: Vec<BatchErrorData>,
}

/// Individual batch error
#[derive(Debug, Clone, Deserialize)]
pub struct BatchErrorData {
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
    pub line: Option<usize>,
}

/// Batch request counts
#[derive(Debug, Clone, Deserialize)]
pub struct BatchRequestCounts {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
}

/// `JSONL` line for batch processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlBatchLine {
    pub custom_id: String,
    pub method: String,
    pub url: String,
    pub body: JsonlRequestBody,
}

/// Request body for `JSONL` batch line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonlRequestBody {
    pub model: String,
    pub input: String,
    pub encoding_format: String,
    pub dimensions: Option<u32>,
}

/// `JSONL` response line from batch processing
#[derive(Debug, Clone, Deserialize)]
pub struct JsonlResponseLine {
    pub id: String,
    pub custom_id: String,
    pub response: JsonlResponse,
    pub error: Option<JsonlError>,
}

/// Response structure in `JSONL` response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonlResponse {
    pub status_code: u16,
    pub request_id: String,
    pub body: JsonlResponseBody,
}

/// Response body in `JSONL` response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonlResponseBody {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: EmbeddingUsage,
}

/// Error structure in `JSONL` response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonlError {
    pub code: String,
    pub message: String,
}

/// Usage information from embedding API
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// File upload response
#[derive(Debug, Clone, Deserialize)]
pub struct FileUploadResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
}

/// File download/content response
#[derive(Debug, Clone, Deserialize)]
pub struct FileContentResponse {
    pub content: String,
}

// === Cost Tracking Models ===

/// Cost tracking information
#[derive(Debug, Clone)]
pub struct CostInfo {
    pub batch_id: String,
    pub tokens_used: u32,
    pub cost_usd: f64,
    pub individual_cost_usd: f64, // What it would have cost without batching
    pub savings_usd: f64,
    pub savings_percentage: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CostInfo {
    /// Calculate cost information for a batch
    #[must_use]
    pub fn calculate(batch_id: String, tokens_used: u32) -> Self {
        let individual_cost = (f64::from(tokens_used) * EMBEDDING_COST_PER_MILLION_TOKENS) / 1_000_000.0;
        let batch_cost = individual_cost * BATCH_DISCOUNT_FACTOR;
        let savings = individual_cost - batch_cost;
        let savings_percentage = (savings / individual_cost) * 100.0;

        Self {
            batch_id,
            tokens_used,
            cost_usd: batch_cost,
            individual_cost_usd: individual_cost,
            savings_usd: savings,
            savings_percentage,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get the savings percentage as a formatted string
    #[must_use]
    pub fn savings_percentage_formatted(&self) -> String {
        format!("{:.1}%", self.savings_percentage)
    }
}
