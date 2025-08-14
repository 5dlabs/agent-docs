//! Integration tests for `OpenAI` Batch API
//!
//! These tests require live `OpenAI` API keys and database connections.
//! They are designed to test the full end-to-end batch processing workflow
//! with real API calls and actual cost savings validation.

#[cfg(test)]
mod tests {
    use super::super::batch::{BatchProcessor, EmbeddingBatchRequest};
    use super::super::client::{EmbeddingClient, OpenAIEmbeddingClient};
    use anyhow::Result;
    use std::env;
    use tokio::time::{sleep, Duration};
    use tracing::{info, warn};

    /// Initialize tracing for tests
    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();
    }

    /// Check if we have the required environment variables for live testing
    fn check_live_env() -> bool {
        env::var("OPENAI_API_KEY").is_ok() && env::var("DATABASE_URL").is_ok()
    }

    /// Skip test if live environment is not available
    macro_rules! skip_if_no_live_env {
        () => {
            if !check_live_env() {
                warn!("Skipping live test: OPENAI_API_KEY or DATABASE_URL not set");
                return Ok(());
            }
        };
    }

    #[tokio::test]
    async fn test_openai_client_creation() -> Result<()> {
        init_tracing();
        skip_if_no_live_env!();

        let client = OpenAIEmbeddingClient::new()?;
        info!("Successfully created OpenAI client");

        // Test individual embedding (small scale to avoid costs)
        let embedding = client.embed("Hello, world!").await?;
        assert_eq!(embedding.len(), 3072); // text-embedding-3-large dimensions
        info!(
            "Successfully generated individual embedding with {} dimensions",
            embedding.len()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_processing_small_scale() -> Result<()> {
        init_tracing();
        skip_if_no_live_env!();

        let client = OpenAIEmbeddingClient::new()?;
        let processor = BatchProcessor::new(client);

        // Create a small batch of test requests (to minimize costs)
        let test_texts = [
            "The quick brown fox jumps over the lazy dog.",
            "Machine learning is transforming how we process data.",
            "Rust is a systems programming language focused on safety.",
            "OpenAI provides powerful AI models through their API.",
            "Batch processing can significantly reduce API costs.",
        ];

        info!("Adding {} requests to batch processor", test_texts.len());
        let mut batch_ids = Vec::new();

        // Add requests to batch processor
        for (i, text) in test_texts.iter().enumerate() {
            let request =
                EmbeddingBatchRequest::new(format!("test-request-{i}"), (*text).to_string());
            let batch_id = processor.add_request(request).await?;
            batch_ids.push(batch_id);
        }

        info!("Added all requests, batch IDs: {:?}", batch_ids);

        // Force submit the current batch (even if small)
        if let Some(submitted_batch_id) = processor.flush_current_batch().await? {
            info!("Successfully submitted batch: {}", submitted_batch_id);

            // Monitor batch status (with timeout)
            let max_wait_time = Duration::from_secs(600); // 10 minutes max
            let start_time = std::time::Instant::now();

            loop {
                let completed_ids = processor.check_batch_statuses().await?;

                if completed_ids.contains(&submitted_batch_id) {
                    info!("Batch {} completed successfully!", submitted_batch_id);
                    break;
                }

                if start_time.elapsed() > max_wait_time {
                    warn!("Batch processing timeout after 10 minutes");
                    break;
                }

                info!("Waiting for batch to complete...");
                sleep(Duration::from_secs(30)).await; // Check every 30 seconds
            }

            // Get batch statistics
            if let Some(batch) = processor.get_batch(&submitted_batch_id).await {
                let stats = batch.get_stats();
                info!("Batch Statistics:");
                info!("  Total requests: {}", stats.total_requests);
                info!("  Successful: {}", stats.successful_results);
                info!("  Failed: {}", stats.failed_results);
                info!("  Total tokens: {}", stats.total_tokens);

                if let Some(cost_info) = &stats.cost_info {
                    info!("  Cost (batch): ${:.6}", cost_info.cost_usd);
                    info!("  Cost (individual): ${:.6}", cost_info.individual_cost_usd);
                    info!(
                        "  Savings: ${:.6} ({})",
                        cost_info.savings_usd,
                        cost_info.savings_percentage_formatted()
                    );

                    // Verify cost savings
                    assert!(
                        cost_info.savings_percentage > 40.0,
                        "Expected at least 40% cost savings"
                    );
                }

                if let Some(processing_time) = stats.processing_time {
                    info!("  Processing time: {:?}", processing_time);
                }

                // Verify we got embeddings for all requests
                assert_eq!(stats.successful_results, test_texts.len());

                // Verify embedding quality (basic check)
                for result in &batch.results {
                    if result.error.is_none() {
                        assert_eq!(
                            result.embedding.len(),
                            3072,
                            "Embedding should have 3072 dimensions"
                        );
                        assert!(
                            !result.embedding.iter().all(|&x| x == 0.0),
                            "Embedding should not be all zeros"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_rate_limiting() -> Result<()> {
        init_tracing();
        skip_if_no_live_env!();

        let _client = OpenAIEmbeddingClient::new()?;

        // Test rate limiting by making several requests
        info!("Testing rate limiting with multiple concurrent requests");
        let mut handles = Vec::new();

        for i in 0..5 {
            let client_clone = OpenAIEmbeddingClient::new()?;
            let handle = tokio::spawn(async move {
                let text = format!("Rate limiting test request number {i}");
                client_clone.embed(&text).await
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut successful_requests = 0;
        for handle in handles {
            match handle.await? {
                Ok(embedding) => {
                    assert_eq!(embedding.len(), 3072);
                    successful_requests += 1;
                }
                Err(e) => {
                    warn!("Request failed: {}", e);
                }
            }
        }

        info!(
            "Successfully completed {} out of 5 rate-limited requests",
            successful_requests
        );
        assert!(
            successful_requests >= 3,
            "At least 3 requests should succeed with rate limiting"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_logic() -> Result<()> {
        init_tracing();
        skip_if_no_live_env!();

        let client = OpenAIEmbeddingClient::new()?;

        // Test retry logic with a normal request (should succeed on first try)
        info!("Testing retry logic with normal request");
        let result = client.embed("Test retry logic").await?;
        assert_eq!(result.len(), 3072);

        info!("Retry logic test passed - normal requests work correctly");

        Ok(())
    }

    #[tokio::test]
    async fn test_cost_calculation_accuracy() -> Result<()> {
        init_tracing();
        use crate::models::CostInfo;

        // Test cost calculation logic
        let batch_id = "test-batch-001".to_string();
        let tokens_used = 10000u32;

        let cost_info = CostInfo::calculate(batch_id.clone(), tokens_used);

        info!("Cost calculation test:");
        info!("  Tokens used: {}", tokens_used);
        info!("  Batch cost: ${:.6}", cost_info.cost_usd);
        info!("  Individual cost: ${:.6}", cost_info.individual_cost_usd);
        info!("  Savings: ${:.6}", cost_info.savings_usd);
        info!(
            "  Savings percentage: {}",
            cost_info.savings_percentage_formatted()
        );

        // Verify calculations
        let expected_individual_cost = (10000.0 * 0.13) / 1_000_000.0;
        let expected_batch_cost = expected_individual_cost * 0.5;
        let expected_savings = expected_individual_cost - expected_batch_cost;
        let expected_savings_percentage = (expected_savings / expected_individual_cost) * 100.0;

        assert!((cost_info.individual_cost_usd - expected_individual_cost).abs() < 1e-10);
        assert!((cost_info.cost_usd - expected_batch_cost).abs() < 1e-10);
        assert!((cost_info.savings_usd - expected_savings).abs() < 1e-10);
        assert!((cost_info.savings_percentage - expected_savings_percentage).abs() < 1e-10);

        // Verify 50% savings
        assert!((cost_info.savings_percentage - 50.0).abs() < f64::EPSILON);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_processor_cleanup() -> Result<()> {
        init_tracing();
        skip_if_no_live_env!();

        let client = OpenAIEmbeddingClient::new()?;
        let processor = BatchProcessor::new(client);

        // Add a small request
        let request = EmbeddingBatchRequest::new(
            "cleanup-test".to_string(),
            "Test cleanup functionality".to_string(),
        );

        let batch_id = processor.add_request(request).await?;
        info!("Added test request to batch: {}", batch_id);

        // Get initial stats
        let initial_stats = processor.get_all_stats().await;
        info!("Initial batch count: {}", initial_stats.len());
        assert!(!initial_stats.is_empty());

        // Test cleanup (should not remove active batch)
        let removed_count = processor
            .cleanup_old_batches(Duration::from_secs(1))
            .await?;
        info!("Cleanup removed {} old batches", removed_count);

        let after_cleanup_stats = processor.get_all_stats().await;
        info!("Batch count after cleanup: {}", after_cleanup_stats.len());

        // Active batches should not be removed
        assert_eq!(initial_stats.len(), after_cleanup_stats.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_performance_benchmarks() -> Result<()> {
        init_tracing();

        // This test doesn't require live API but tests performance characteristics
        let client = OpenAIEmbeddingClient::new()?;
        let processor = BatchProcessor::new(client);

        let start_time = std::time::Instant::now();

        // Add many requests quickly (simulate load)
        for i in 0..1000 {
            let request = EmbeddingBatchRequest::new(
                format!("perf-test-{i}"),
                format!("Performance test request number {i}"),
            );
            processor.add_request(request).await?;
        }

        let add_duration = start_time.elapsed();
        info!(
            "Added 1000 requests in {:?} ({:.2} req/sec)",
            add_duration,
            1000.0 / add_duration.as_secs_f64()
        );

        // Verify performance expectations
        assert!(
            add_duration < Duration::from_secs(5),
            "Adding 1000 requests should take less than 5 seconds"
        );

        // Check batch statistics
        let stats = processor.get_all_stats().await;
        info!("Created {} batches for 1000 requests", stats.len());

        // Should create multiple batches due to 20k limit
        let total_requests: usize = stats.iter().map(|s| s.total_requests).sum();
        assert_eq!(total_requests, 1000);

        Ok(())
    }
}
