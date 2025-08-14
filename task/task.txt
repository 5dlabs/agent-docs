# Task ID: 8
# Title: OpenAI Embedding Client with Batch Processing
# Status: pending
# Dependencies: 6
# Priority: medium
# Description: Implement optimized OpenAI embedding generation using text-embedding-3-large model with batch processing for 50% cost reduction.
# Details:
Implement batch processing using OpenAI Batch API with JSONL format. Configure text-embedding-3-large with 3072 dimensions (full) or 1024 (optimized). Create batch queue with 20,000 line chunks for optimal performance. Implement rate limiting (3000 RPM / 1M TPM) with token bucket algorithm. Add retry logic with exponential backoff for failed batches. Store API keys securely using environment variables. Implement cost tracking and reporting. Use reqwest 0.12 for HTTP client with rustls-tls.

# Test Strategy:
Test batch creation and submission, validate embedding dimensions and quality, measure cost reduction compared to individual requests, test rate limiting under high load, verify retry mechanisms, and benchmark processing time (target < 20 minutes for 20k embeddings).

# Subtasks:
## 1. Create OpenAI Batch API Models [pending]
### Dependencies: None
### Description: Define data structures for OpenAI Batch API requests and responses including JSONL format support
### Details:
Create structs in crates/embeddings/src/models.rs for BatchRequest, BatchJob, BatchStatus, and JSONL line items. Define enums for batch status (validating, in_progress, finalizing, completed, failed, expired, cancelling, cancelled). Add serialization/deserialization support with serde for JSONL format. Include fields for custom_id, method, url, and body for each batch line item.

## 2. Implement Rate Limiting with Token Bucket [pending]
### Dependencies: None
### Description: Build token bucket algorithm for OpenAI API rate limiting (3000 RPM / 1M TPM)
### Details:
Create RateLimiter struct in crates/embeddings/src/client.rs with token bucket implementation. Track both requests per minute (3000) and tokens per minute (1M). Implement async wait mechanism when buckets are empty. Add methods to consume tokens based on text length. Use tokio::time for timing and tokio::sync::Mutex for thread-safe token management. Include burst capacity handling.

## 3. Build Batch Processing Queue System [pending]
### Dependencies: 7.1
### Description: Implement batch queue manager for processing documents in 20,000 line chunks with JSONL format
### Details:
Enhance crates/embeddings/src/batch.rs with BatchQueue struct. Implement queue management for accumulating embedding requests. Add chunking logic to split into 20,000 line batches. Create JSONL file generation with proper formatting. Implement async batch submission to OpenAI Batch API. Add queue persistence for recovery. Track batch IDs and statuses.

## 4. Add Retry Logic and Error Handling [pending]
### Dependencies: 7.2, 7.3
### Description: Implement exponential backoff retry mechanism for failed batch operations
### Details:
Create RetryPolicy struct with exponential backoff configuration (base: 1s, max: 60s, max_retries: 5). Implement retry logic for batch submission, status checking, and result retrieval. Add error classification (retryable vs permanent). Handle specific OpenAI error codes. Implement circuit breaker pattern for repeated failures. Add detailed error logging with context.

## 5. Implement Cost Tracking and Reporting [pending]
### Dependencies: 7.3, 7.4
### Description: Build cost tracking system for monitoring embedding generation expenses and validating 50% cost reduction
### Details:
Create CostTracker struct to monitor API usage and costs. Track tokens consumed per batch (text-embedding-3-large pricing: $0.13 per 1M tokens). Compare batch API costs (50% discount) vs regular API. Store cost data in database with batch_id, tokens_used, cost_usd, timestamp. Generate cost reports by time period. Add alerts for cost thresholds. Calculate and display actual savings percentage.

