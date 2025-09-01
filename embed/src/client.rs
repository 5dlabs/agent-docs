//! `OpenAI` embedding client

use crate::models::{
    BatchRequest, BatchResponse, EmbeddingRequest, EmbeddingResponse, FileUploadResponse,
    JsonlResponseLine,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{multipart, Client};
use serde_json::json;
use std::{env, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};
use tracing::{debug, error, info, warn};

// === Retry Logic Configuration ===

/// Retry policy for `OpenAI` API operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Base delay for exponential backoff
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor to add randomness (0.0 to 1.0)
    pub jitter_factor: f64,
}

impl RetryPolicy {
    /// Create a new retry policy with OpenAI-optimized defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_retries: 5,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }

    /// Calculate delay for a specific retry attempt
    #[must_use]
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let exponential_delay =
            self.base_delay.as_secs_f64() * self.backoff_multiplier.powi((attempt - 1) as i32);
        let clamped_delay = exponential_delay.min(self.max_delay.as_secs_f64());

        // Add jitter
        let jitter_range = clamped_delay * self.jitter_factor;
        let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
        let final_delay = (clamped_delay + jitter).max(0.0);

        Duration::from_secs_f64(final_delay)
    }

    /// Check if an error is retryable
    #[must_use]
    pub fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_string = error.to_string().to_lowercase();

        // Check for temporary network/server errors
        error_string.contains("timeout") ||
        error_string.contains("connection") ||
        error_string.contains("network") ||
        error_string.contains("temporary") ||
        error_string.contains("503") ||  // Service Unavailable
        error_string.contains("502") ||  // Bad Gateway
        error_string.contains("504") ||  // Gateway Timeout
        error_string.contains("429") ||  // Too Many Requests
        error_string.contains("500") // Internal Server Error
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Circuit breaker to prevent cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of consecutive failures
    failure_count: u32,
    /// Maximum failures before opening circuit
    failure_threshold: u32,
    /// Time to wait before trying again after circuit opens
    recovery_timeout: Duration,
    /// Last failure time
    last_failure: Option<Instant>,
    /// Current state
    state: CircuitBreakerState,
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    #[must_use]
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_count: 0,
            failure_threshold,
            recovery_timeout,
            last_failure: None,
            state: CircuitBreakerState::Closed,
        }
    }

    /// Check if a request can proceed
    #[must_use]
    pub fn can_proceed(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    // No last failure recorded, allow request
                    self.state = CircuitBreakerState::Closed;
                    true
                }
            }
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitBreakerState::Closed;
        self.last_failure = None;
    }

    /// Record a failed operation
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitBreakerState::Open;
            warn!(
                "Circuit breaker opened after {} failures",
                self.failure_count
            );
        }
    }
}

// === Rate Limiting Configuration ===

/// `OpenAI` API rate limits for embeddings
const OPENAI_RPM_LIMIT: u32 = 3000; // Requests per minute
const OPENAI_TPM_LIMIT: u32 = 1_000_000; // Tokens per minute
const RATE_LIMIT_WINDOW_SECS: u64 = 60; // 1 minute window
const AVERAGE_TOKENS_PER_CHAR: f64 = 0.25; // Rough estimate for tokenization

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    /// Current token count
    tokens: f64,
    /// Maximum tokens in bucket
    capacity: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume tokens, return false if not enough available
    fn try_consume(&mut self, amount: f64) -> bool {
        self.refill();
        if self.tokens >= amount {
            self.tokens -= amount;
            true
        } else {
            false
        }
    }

    /// Get time to wait until we can consume the requested amount
    fn time_to_availability(&mut self, amount: f64) -> Duration {
        self.refill();
        if self.tokens >= amount {
            Duration::ZERO
        } else {
            let needed_tokens = amount - self.tokens;
            let wait_time = needed_tokens / self.refill_rate;
            Duration::from_secs_f64(wait_time)
        }
    }
}

/// Rate limiter for `OpenAI` API calls
#[derive(Debug)]
pub struct RateLimiter {
    /// Bucket for requests per minute
    request_bucket: Arc<Mutex<TokenBucket>>,
    /// Bucket for tokens per minute
    token_bucket: Arc<Mutex<TokenBucket>>,
}

impl RateLimiter {
    /// Create a new rate limiter with `OpenAI` API limits
    #[must_use]
    pub fn new() -> Self {
        #[allow(clippy::cast_precision_loss)]
        let rpm_rate = f64::from(OPENAI_RPM_LIMIT) / (RATE_LIMIT_WINDOW_SECS as f64);
        #[allow(clippy::cast_precision_loss)]
        let tpm_rate = f64::from(OPENAI_TPM_LIMIT) / (RATE_LIMIT_WINDOW_SECS as f64);

        Self {
            request_bucket: Arc::new(Mutex::new(TokenBucket::new(
                f64::from(OPENAI_RPM_LIMIT),
                rpm_rate,
            ))),
            token_bucket: Arc::new(Mutex::new(TokenBucket::new(
                f64::from(OPENAI_TPM_LIMIT),
                tpm_rate,
            ))),
        }
    }

    /// Wait until we can make a request with the given token count
    ///
    /// # Errors
    ///
    /// Returns an error if rate limiting fails (should not happen in normal operation).
    pub async fn wait_for_capacity(&self, estimated_tokens: u32) -> Result<()> {
        let tokens = f64::from(estimated_tokens);

        loop {
            let mut request_bucket = self.request_bucket.lock().await;
            let mut token_bucket = self.token_bucket.lock().await;

            // Check if we can consume both a request and the required tokens
            if request_bucket.try_consume(1.0) && token_bucket.try_consume(tokens) {
                debug!(
                    "Rate limit check passed: 1 request, {} tokens",
                    estimated_tokens
                );
                break;
            }

            // Calculate how long we need to wait
            let request_wait = request_bucket.time_to_availability(1.0);
            let token_wait = token_bucket.time_to_availability(tokens);
            let wait_duration = request_wait.max(token_wait);

            drop(request_bucket);
            drop(token_bucket);

            if wait_duration > Duration::ZERO {
                warn!("Rate limit hit, waiting {:?} before retry", wait_duration);
                tokio::time::sleep(wait_duration).await;
            } else {
                // Small backoff to avoid busy loop
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        Ok(())
    }

    /// Estimate token count from text length
    #[must_use]
    pub fn estimate_tokens(text: &str) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let estimated = (text.len() as f64 * AVERAGE_TOKENS_PER_CHAR).ceil() as u32;
        estimated.max(1) // At least 1 token
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for embedding clients
#[async_trait]
pub trait EmbeddingClient {
    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embedding using the client's API
    async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Upload a `JSONL` file for batch processing
    async fn upload_batch_file(&self, content: &str, filename: &str) -> Result<FileUploadResponse>;

    /// Create a batch job
    async fn create_batch(&self, input_file_id: &str) -> Result<BatchResponse>;

    /// Get batch status
    async fn get_batch(&self, batch_id: &str) -> Result<BatchResponse>;

    /// Download batch results
    async fn download_batch_results(&self, file_id: &str) -> Result<Vec<JsonlResponseLine>>;

    /// Cancel a batch
    async fn cancel_batch(&self, batch_id: &str) -> Result<BatchResponse>;
}

/// `OpenAI` embedding client implementation
pub struct OpenAIEmbeddingClient {
    client: Client,
    api_key: String,
    rate_limiter: RateLimiter,
    retry_policy: RetryPolicy,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl OpenAIEmbeddingClient {
    /// Create a new embedding client
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables or HTTP client
    /// initialization fails.
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| "dummy-key".to_string()); // Allow dummy key for testing

        let client = Client::new();

        Ok(Self {
            client,
            api_key,
            rate_limiter: RateLimiter::new(),
            retry_policy: RetryPolicy::new(),
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(5, Duration::from_secs(300)))), // 5 failures, 5-minute timeout
        })
    }

    /// Execute an operation with retry logic and circuit breaker
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check circuit breaker
        {
            let mut circuit_breaker = self.circuit_breaker.lock().await;
            if !circuit_breaker.can_proceed() {
                return Err(anyhow!(
                    "Circuit breaker is open - too many recent failures"
                ));
            }
        }

        let mut last_error = None;

        for attempt in 0..=self.retry_policy.max_retries {
            match operation().await {
                Ok(result) => {
                    // Record success and return
                    {
                        let mut circuit_breaker = self.circuit_breaker.lock().await;
                        circuit_breaker.record_success();
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);

                    // Check if this is the last attempt
                    if attempt == self.retry_policy.max_retries {
                        break;
                    }

                    // Check if error is retryable
                    if let Some(ref err) = last_error {
                        if !RetryPolicy::is_retryable_error(err) {
                            debug!("Error is not retryable, failing immediately: {}", err);
                            break;
                        }
                    }

                    // Calculate delay and wait
                    let delay = self.retry_policy.calculate_delay(attempt + 1);
                    warn!(
                        "Request failed (attempt {}/{}), retrying after {:?}: {}",
                        attempt + 1,
                        self.retry_policy.max_retries + 1,
                        delay,
                        last_error.as_ref().map_or(
                            "Unknown error".to_string(),
                            std::string::ToString::to_string
                        )
                    );

                    if delay > Duration::ZERO {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All retries failed, record failure and return error
        {
            let mut circuit_breaker = self.circuit_breaker.lock().await;
            circuit_breaker.record_failure();
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
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

    /// Generate embedding using `OpenAI` API
    async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!(
            "Generating embedding for {} characters",
            request.input.len()
        );

        // Apply rate limiting
        let estimated_tokens = RateLimiter::estimate_tokens(&request.input);
        self.rate_limiter
            .wait_for_capacity(estimated_tokens)
            .await?;

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

        #[allow(clippy::cast_possible_truncation)]
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

    /// Upload a `JSONL` file for batch processing
    async fn upload_batch_file(&self, content: &str, filename: &str) -> Result<FileUploadResponse> {
        debug!(
            "Uploading batch file: {} ({} bytes)",
            filename,
            content.len()
        );

        let content = content.to_string();
        let filename = filename.to_string();

        self.execute_with_retry(|| async {
            let form = multipart::Form::new().text("purpose", "batch").part(
                "file",
                multipart::Part::text(content.clone())
                    .file_name(filename.clone())
                    .mime_str("application/jsonl")?,
            );

            let response = self
                .client
                .post("https://api.openai.com/v1/files")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .multipart(form)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                error!("OpenAI file upload error: {}", error_text);
                return Err(anyhow!("OpenAI file upload error: {}", error_text));
            }

            let upload_response: FileUploadResponse = response.json().await?;
            info!("Successfully uploaded file: {}", upload_response.id);
            Ok(upload_response)
        })
        .await
    }

    /// Create a batch job
    async fn create_batch(&self, input_file_id: &str) -> Result<BatchResponse> {
        debug!("Creating batch with input file: {}", input_file_id);

        let input_file_id = input_file_id.to_string();

        self.execute_with_retry(|| async {
            let request = BatchRequest {
                input_file_id: input_file_id.clone(),
                endpoint: "/v1/embeddings".to_string(),
                completion_window: "24h".to_string(),
                metadata: None,
            };

            let response = self
                .client
                .post("https://api.openai.com/v1/batches")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                error!("OpenAI batch creation error: {}", error_text);
                return Err(anyhow!("OpenAI batch creation error: {}", error_text));
            }

            let batch_response: BatchResponse = response.json().await?;
            info!("Successfully created batch: {}", batch_response.id);
            Ok(batch_response)
        })
        .await
    }

    /// Get batch status
    async fn get_batch(&self, batch_id: &str) -> Result<BatchResponse> {
        debug!("Getting batch status: {}", batch_id);

        let response = self
            .client
            .get(format!("https://api.openai.com/v1/batches/{batch_id}"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI batch status error: {}", error_text);
            return Err(anyhow!("OpenAI batch status error: {}", error_text));
        }

        let batch_response: BatchResponse = response.json().await?;
        debug!("Batch {} status: {:?}", batch_id, batch_response.status);
        Ok(batch_response)
    }

    /// Download batch results
    async fn download_batch_results(&self, file_id: &str) -> Result<Vec<JsonlResponseLine>> {
        debug!("Downloading batch results from file: {}", file_id);

        let response = self
            .client
            .get(format!("https://api.openai.com/v1/files/{file_id}/content"))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI file download error: {}", error_text);
            return Err(anyhow!("OpenAI file download error: {}", error_text));
        }

        let content = response.text().await?;
        let mut results = Vec::new();

        for line in content.lines() {
            if !line.trim().is_empty() {
                match serde_json::from_str::<JsonlResponseLine>(line) {
                    Ok(response_line) => results.push(response_line),
                    Err(e) => {
                        warn!("Failed to parse JSONL line: {} - Error: {}", line, e);
                    }
                }
            }
        }

        info!("Downloaded {} batch results", results.len());
        Ok(results)
    }

    /// Cancel a batch
    async fn cancel_batch(&self, batch_id: &str) -> Result<BatchResponse> {
        debug!("Cancelling batch: {}", batch_id);

        let response = self
            .client
            .post(format!(
                "https://api.openai.com/v1/batches/{batch_id}/cancel"
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI batch cancellation error: {}", error_text);
            return Err(anyhow!("OpenAI batch cancellation error: {}", error_text));
        }

        let batch_response: BatchResponse = response.json().await?;
        info!("Successfully cancelled batch: {}", batch_response.id);
        Ok(batch_response)
    }
}
