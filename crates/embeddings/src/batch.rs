//! Batch processing for embeddings
//!
//! This module provides a queue-based system for processing embedding requests
//! in batches through the `OpenAI` Batch API, enabling significant cost savings.

use crate::client::{EmbeddingClient, RateLimiter};
use crate::models::{
    BatchResponse, BatchStatus, CostInfo, JsonlBatchLine, JsonlRequestBody, JsonlResponseLine,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use tracing::{debug, error, info, warn};

/// Maximum number of requests per batch (`OpenAI` limit)
const MAX_BATCH_SIZE: usize = 50_000;
/// Optimal batch size for cost-performance balance
const OPTIMAL_BATCH_SIZE: usize = 20_000;
/// Maximum wait time for batch to fill before processing
const MAX_BATCH_WAIT_TIME: Duration = Duration::from_secs(300); // 5 minutes

/// A batch embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingBatchRequest {
    /// Unique identifier for this request
    pub id: String,
    /// Text to embed
    pub text: String,
    /// Model to use for embedding
    pub model: String,
    /// Optional dimensions (for text-embedding-3-large)
    pub dimensions: Option<u32>,
    /// Metadata associated with this request
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EmbeddingBatchRequest {
    /// Create a new batch request with text-embedding-3-large defaults
    #[must_use]
    pub fn new(id: String, text: String) -> Self {
        Self {
            id,
            text,
            model: "text-embedding-3-large".to_string(),
            dimensions: Some(3072), // Full dimensionality
            metadata: HashMap::new(),
        }
    }

    /// Create a new batch request with optimized dimensions
    #[must_use]
    pub fn new_optimized(id: String, text: String) -> Self {
        Self {
            id,
            text,
            model: "text-embedding-3-large".to_string(),
            dimensions: Some(1024), // Optimized dimensionality for better performance
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the request
    #[must_use]
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Convert to `JSONL` format for batch processing
    #[must_use]
    pub fn to_jsonl_line(&self) -> JsonlBatchLine {
        JsonlBatchLine {
            custom_id: self.id.clone(),
            method: "POST".to_string(),
            url: "/v1/embeddings".to_string(),
            body: JsonlRequestBody {
                model: self.model.clone(),
                input: self.text.clone(),
                encoding_format: "float".to_string(),
                dimensions: self.dimensions,
            },
        }
    }

    /// Estimate token count for this request
    #[must_use]
    pub fn estimate_tokens(&self) -> u32 {
        RateLimiter::estimate_tokens(&self.text)
    }
}

/// Result of a batch embedding operation
#[derive(Debug, Clone)]
pub struct EmbeddingBatchResult {
    /// Original request ID
    pub request_id: String,
    /// Generated embedding vector
    pub embedding: Vec<f32>,
    /// Token count used
    pub tokens_used: u32,
    /// Any error that occurred
    pub error: Option<String>,
}

/// Status of a batch operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchQueueStatus {
    /// Accepting new requests
    Accepting,
    /// Batch is full or timeout reached, ready for processing
    Ready,
    /// Batch has been submitted to `OpenAI`
    Submitted,
    /// Batch is being processed by `OpenAI`
    Processing,
    /// Batch processing completed successfully
    Completed,
    /// Batch processing failed
    Failed,
    /// Batch was cancelled
    Cancelled,
}

/// A batch of embedding requests
#[derive(Debug, Clone)]
pub struct EmbeddingBatch {
    /// Batch identifier
    pub id: String,
    /// Requests in this batch
    pub requests: Vec<EmbeddingBatchRequest>,
    /// Current status
    pub status: BatchQueueStatus,
    /// `OpenAI` batch ID (once submitted)
    pub openai_batch_id: Option<String>,
    /// When this batch was created
    pub created_at: Instant,
    /// When this batch was submitted
    pub submitted_at: Option<Instant>,
    /// When this batch was completed
    pub completed_at: Option<Instant>,
    /// Results from batch processing
    pub results: Vec<EmbeddingBatchResult>,
    /// Cost information
    pub cost_info: Option<CostInfo>,
}

impl EmbeddingBatch {
    /// Create a new batch
    #[must_use]
    pub fn new(id: String) -> Self {
        Self {
            id,
            requests: Vec::new(),
            status: BatchQueueStatus::Accepting,
            openai_batch_id: None,
            created_at: Instant::now(),
            submitted_at: None,
            completed_at: None,
            results: Vec::new(),
            cost_info: None,
        }
    }

    /// Check if batch can accept more requests
    #[must_use]
    pub fn can_accept_requests(&self) -> bool {
        self.status == BatchQueueStatus::Accepting && self.requests.len() < MAX_BATCH_SIZE
    }

    /// Check if batch is ready for processing
    #[must_use]
    pub fn is_ready_for_processing(&self) -> bool {
        match self.status {
            BatchQueueStatus::Accepting => {
                self.requests.len() >= OPTIMAL_BATCH_SIZE
                    || self.created_at.elapsed() >= MAX_BATCH_WAIT_TIME
            }
            BatchQueueStatus::Ready => true,
            _ => false,
        }
    }

    /// Add a request to this batch
    ///
    /// # Errors
    ///
    /// Returns an error if the batch cannot accept more requests.
    pub fn add_request(&mut self, request: EmbeddingBatchRequest) -> Result<()> {
        if !self.can_accept_requests() {
            return Err(anyhow!(
                "Batch {} cannot accept more requests (status: {:?}, size: {})",
                self.id,
                self.status,
                self.requests.len()
            ));
        }

        self.requests.push(request);
        
        // Check if we should mark as ready
        if self.is_ready_for_processing() && self.status == BatchQueueStatus::Accepting {
            self.status = BatchQueueStatus::Ready;
            debug!("Batch {} marked as ready with {} requests", self.id, self.requests.len());
        }

        Ok(())
    }

    /// Mark batch as submitted
    pub fn mark_submitted(&mut self, openai_batch_id: String) {
        self.openai_batch_id = Some(openai_batch_id);
        self.status = BatchQueueStatus::Submitted;
        self.submitted_at = Some(Instant::now());
    }

    /// Update batch status from `OpenAI` response
    pub fn update_from_openai_response(&mut self, response: &BatchResponse) {
        self.status = match response.status {
            BatchStatus::Validating | BatchStatus::InProgress | BatchStatus::Finalizing => {
                BatchQueueStatus::Processing
            }
            BatchStatus::Completed => BatchQueueStatus::Completed,
            BatchStatus::Failed | BatchStatus::Expired => BatchQueueStatus::Failed,
            BatchStatus::Cancelled | BatchStatus::Cancelling => BatchQueueStatus::Cancelled,
        };

        if self.status == BatchQueueStatus::Completed {
            self.completed_at = Some(Instant::now());
        }
    }

    /// Process batch results and calculate costs
    ///
    /// # Errors
    ///
    /// Returns an error if result processing fails (should not happen in normal operation).
    pub fn process_results(&mut self, results: Vec<JsonlResponseLine>) -> Result<()> {
        self.results.clear();
        let mut total_tokens = 0u32;

        for result_line in results {
            let request_id = result_line.custom_id;
            
            match result_line.error {
                Some(error) => {
                    // Handle error case
                    self.results.push(EmbeddingBatchResult {
                        request_id: request_id.clone(),
                        embedding: Vec::new(),
                        tokens_used: 0,
                        error: Some(format!("{}: {}", error.code, error.message)),
                    });
                    warn!("Batch request {} failed: {}", request_id, error.message);
                }
                None => {
                    // Handle success case
                    if let Some(embedding_data) = result_line.response.body.data.first() {
                        let tokens_used = result_line.response.body.usage.total_tokens;
                        total_tokens += tokens_used;

                        self.results.push(EmbeddingBatchResult {
                            request_id: request_id.clone(),
                            embedding: embedding_data.embedding.clone(),
                            tokens_used,
                            error: None,
                        });
                    } else {
                        self.results.push(EmbeddingBatchResult {
                            request_id: request_id.clone(),
                            embedding: Vec::new(),
                            tokens_used: 0,
                            error: Some("No embedding data in response".to_string()),
                        });
                    }
                }
            }
        }

        // Calculate cost information
        if total_tokens > 0 {
            self.cost_info = Some(CostInfo::calculate(
                self.openai_batch_id.clone().unwrap_or_else(|| self.id.clone()),
                total_tokens,
            ));
        }

        info!(
            "Processed {} results for batch {}, total tokens: {}, cost savings: {}",
            self.results.len(),
            self.id,
            total_tokens,
            self.cost_info
                .as_ref()
                .map_or("N/A".to_string(), CostInfo::savings_percentage_formatted)
        );

        Ok(())
    }

    /// Generate `JSONL` content for batch submission
    #[must_use]
    pub fn generate_jsonl_content(&self) -> String {
        self.requests
            .iter()
            .map(|req| serde_json::to_string(&req.to_jsonl_line()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get batch statistics
    #[must_use]
    pub fn get_stats(&self) -> BatchStats {
        let total_requests = self.requests.len();
        let successful_results = self.results.iter().filter(|r| r.error.is_none()).count();
        let failed_results = self.results.iter().filter(|r| r.error.is_some()).count();
        let total_tokens: u32 = self.results.iter().map(|r| r.tokens_used).sum();

        BatchStats {
            batch_id: self.id.clone(),
            total_requests,
            successful_results,
            failed_results,
            total_tokens,
            cost_info: self.cost_info.clone(),
            processing_time: self
                .completed_at
                .and_then(|completed| self.submitted_at.map(|submitted| completed.duration_since(submitted))),
        }
    }
}

/// Statistics for a batch operation
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Batch identifier
    pub batch_id: String,
    /// Total requests in batch
    pub total_requests: usize,
    /// Number of successful results
    pub successful_results: usize,
    /// Number of failed results
    pub failed_results: usize,
    /// Total tokens used
    pub total_tokens: u32,
    /// Cost information
    pub cost_info: Option<CostInfo>,
    /// Processing time
    pub processing_time: Option<Duration>,
}

/// Queue-based batch processor for embeddings
pub struct BatchProcessor<T: EmbeddingClient> {
    /// Embedding client
    client: T,
    /// Active batches
    batches: Mutex<HashMap<String, EmbeddingBatch>>,
    /// Current batch being filled
    current_batch: Mutex<Option<String>>,
    /// Batch ID counter
    batch_counter: Mutex<u64>,
}

impl<T: EmbeddingClient + Send + Sync> BatchProcessor<T> {
    /// Create a new batch processor
    #[must_use]
    pub fn new(client: T) -> Self {
        Self {
            client,
            batches: Mutex::new(HashMap::new()),
            current_batch: Mutex::new(None),
            batch_counter: Mutex::new(0),
        }
    }

    /// Add a request to the current batch
    ///
    /// # Errors
    ///
    /// Returns an error if request processing fails.
    pub async fn add_request(&self, request: EmbeddingBatchRequest) -> Result<String> {
        let mut batches = self.batches.lock().await;
        let mut current_batch_id = self.current_batch.lock().await;

        // Get or create current batch
        let batch_id = match current_batch_id.as_ref() {
            Some(id) if batches.get(id).is_some_and(EmbeddingBatch::can_accept_requests) => id.clone(),
            _ => {
                // Create new batch
                let mut counter = self.batch_counter.lock().await;
                *counter += 1;
                let new_id = format!("batch-{:08}", *counter);
                batches.insert(new_id.clone(), EmbeddingBatch::new(new_id.clone()));
                *current_batch_id = Some(new_id.clone());
                new_id
            }
        };

        // Add request to batch
        if let Some(batch) = batches.get_mut(&batch_id) {
            batch.add_request(request)?;
            debug!("Added request to batch {}, size: {}", batch_id, batch.requests.len());
        }

        // Check if batch is ready and should be submitted
        if batches.get(&batch_id).is_some_and(EmbeddingBatch::is_ready_for_processing) {
            *current_batch_id = None; // Force new batch for next request
        }

        Ok(batch_id)
    }

    /// Submit ready batches for processing
    ///
    /// # Errors
    ///
    /// Returns an error if batch submission fails.
    pub async fn submit_ready_batches(&self) -> Result<Vec<String>> {
        let mut batches = self.batches.lock().await;
        let mut submitted_ids = Vec::new();

        let ready_batch_ids: Vec<String> = batches
            .values()
            .filter(|&b| b.is_ready_for_processing())
            .map(|b| b.id.clone())
            .collect();

        for batch_id in ready_batch_ids {
            if let Some(batch) = batches.get_mut(&batch_id) {
                if batch.status == BatchQueueStatus::Ready || batch.status == BatchQueueStatus::Accepting {
                    batch.status = BatchQueueStatus::Ready;
                    
                    // Generate JSONL content and submit
                    let jsonl_content = batch.generate_jsonl_content();
                    let filename = format!("{batch_id}.jsonl");

                    match self.client.upload_batch_file(&jsonl_content, &filename).await {
                        Ok(upload_response) => {
                            match self.client.create_batch(&upload_response.id).await {
                                Ok(batch_response) => {
                                    batch.mark_submitted(batch_response.id.clone());
                                    submitted_ids.push(batch_id.clone());
                                    info!("Successfully submitted batch {}", batch_id);
                                }
                                Err(e) => {
                                    error!("Failed to create batch {}: {}", batch_id, e);
                                    batch.status = BatchQueueStatus::Failed;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to upload file for batch {}: {}", batch_id, e);
                            batch.status = BatchQueueStatus::Failed;
                        }
                    }
                }
            }
        }

        Ok(submitted_ids)
    }

    /// Check status of all active batches
    ///
    /// # Errors
    ///
    /// Returns an error if status checking fails.
    pub async fn check_batch_statuses(&self) -> Result<Vec<String>> {
        let mut batches = self.batches.lock().await;
        let mut completed_ids = Vec::new();

        let active_batches: Vec<(String, String)> = batches
            .values()
            .filter_map(|b| {
                if matches!(b.status, BatchQueueStatus::Submitted | BatchQueueStatus::Processing) {
                    b.openai_batch_id.as_ref().map(|oid| (b.id.clone(), oid.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (batch_id, openai_batch_id) in active_batches {
            match self.client.get_batch(&openai_batch_id).await {
                Ok(response) => {
                    if let Some(batch) = batches.get_mut(&batch_id) {
                        let old_status = batch.status.clone();
                        batch.update_from_openai_response(&response);

                        if batch.status != old_status {
                            debug!("Batch {} status changed: {:?} -> {:?}", batch_id, old_status, batch.status);
                        }

                        // Download results if completed
                        if batch.status == BatchQueueStatus::Completed {
                            if let Some(output_file_id) = &response.output_file_id {
                                match self.client.download_batch_results(output_file_id).await {
                                    Ok(results) => {
                                        if let Err(e) = batch.process_results(results) {
                                            error!("Failed to process results for batch {batch_id}: {e}");
                                        } else {
                                            completed_ids.push(batch_id.clone());
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to download results for batch {batch_id}: {e}");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check status of batch {batch_id}: {e}");
                }
            }
        }

        Ok(completed_ids)
    }

    /// Get batch by ID
    pub async fn get_batch(&self, batch_id: &str) -> Option<EmbeddingBatch> {
        self.batches.lock().await.get(batch_id).cloned()
    }

    /// Get all batch statistics
    pub async fn get_all_stats(&self) -> Vec<BatchStats> {
        self.batches.lock().await.values().map(EmbeddingBatch::get_stats).collect()
    }

    /// Clean up old completed batches
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup fails.
    pub async fn cleanup_old_batches(&self, max_age: Duration) -> Result<usize> {
        let mut batches = self.batches.lock().await;
        let cutoff_time = Instant::now() - max_age;
        
        let old_batch_ids: Vec<String> = batches
            .values()
            .filter(|b| {
                matches!(
                    b.status,
                    BatchQueueStatus::Completed | BatchQueueStatus::Failed | BatchQueueStatus::Cancelled
                ) && b.created_at < cutoff_time
            })
            .map(|b| b.id.clone())
            .collect();

        let removed_count = old_batch_ids.len();
        for batch_id in old_batch_ids {
            batches.remove(&batch_id);
        }

        info!("Cleaned up {removed_count} old batches");
        Ok(removed_count)
    }

    /// Force submit current batch even if not full
    ///
    /// # Errors
    ///
    /// Returns an error if batch submission fails.
    pub async fn flush_current_batch(&self) -> Result<Option<String>> {
        let mut current_batch_id = self.current_batch.lock().await;
        if let Some(batch_id) = current_batch_id.take() {
            let mut batches = self.batches.lock().await;
            if let Some(batch) = batches.get_mut(&batch_id) {
                if batch.status == BatchQueueStatus::Accepting && !batch.requests.is_empty() {
                    batch.status = BatchQueueStatus::Ready;
                    drop(batches);
                    drop(current_batch_id);
                    
                    self.submit_ready_batches().await?;
                    return Ok(Some(batch_id));
                }
            }
        }
        Ok(None)
    }
}