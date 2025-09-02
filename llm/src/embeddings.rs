//! High-level embedding service that integrates LLM client with batch processing
//!
//! This module provides a unified interface for embedding operations,
//! combining the LLM client's direct embedding capabilities with the
//! existing embed crate's sophisticated batch processing features.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::{EmbeddingUseCase, LlmClient};
use crate::models::LlmProvider;

/// Service for handling embedding operations with both direct and batch processing
#[derive(Clone)]
pub struct EmbeddingService {
    /// LLM client for direct embedding operations
    llm_client: LlmClient,
    /// Optional batch processor for large-scale operations
    batch_processor: Option<Arc<dyn BatchEmbeddingProcessor>>,
}

/// Trait for batch embedding processors
#[async_trait::async_trait]
pub trait BatchEmbeddingProcessor: Send + Sync {
    /// Process a batch of texts and return embeddings
    async fn process_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    /// Get batch statistics
    fn get_batch_stats(&self) -> BatchStats;
}

/// Batch processing statistics
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Total texts processed
    pub total_processed: usize,
    /// Number of successful embeddings
    pub successful: usize,
    /// Number of failed embeddings
    pub failed: usize,
    /// Average processing time per text
    pub avg_processing_time_ms: f64,
    /// Cost information if available
    pub cost_info: Option<CostInfo>,
}

/// Cost tracking information
#[derive(Debug, Clone)]
pub struct CostInfo {
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Cost per 1M tokens
    pub cost_per_million_tokens: f64,
    /// Tokens used
    pub tokens_used: u32,
}

impl EmbeddingService {
    /// Create a new embedding service with LLM client
    #[must_use]
    pub fn new(llm_client: LlmClient) -> Self {
        Self {
            llm_client,
            batch_processor: None,
        }
    }

    /// Create embedding service with batch processor
    #[must_use]
    pub fn with_batch_processor(
        llm_client: LlmClient,
        batch_processor: Arc<dyn BatchEmbeddingProcessor>,
    ) -> Self {
        Self {
            llm_client,
            batch_processor: Some(batch_processor),
        }
    }

    /// Generate embedding for a single text
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        self.llm_client.generate_embedding(text).await
    }

    /// Generate optimized embedding for specific use case
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn embed_text_optimized(
        &self,
        text: &str,
        use_case: EmbeddingUseCase,
    ) -> Result<Vec<f32>> {
        self.llm_client
            .generate_embedding_optimized(text, use_case)
            .await
    }

    /// Generate embeddings for multiple texts
    ///
    /// # Errors
    ///
    /// Returns an error if any embedding generation fails.
    pub async fn embed_texts(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Use batch processor if available and texts count is large enough
        if let Some(processor) = &self.batch_processor {
            if texts.len() >= 10 {
                // Threshold for using batch processing
                return processor.process_batch(texts.to_vec()).await;
            }
        }

        // Fall back to individual processing
        self.llm_client.generate_embeddings_batch(texts).await
    }

    /// Calculate semantic similarity between two texts
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn calculate_similarity(&self, text1: &str, text2: &str) -> Result<f32> {
        self.llm_client.calculate_similarity(text1, text2).await
    }

    /// Find most similar texts from a corpus
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn find_similar_texts(
        &self,
        query: &str,
        corpus: &[String],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        self.llm_client
            .find_most_similar(query, corpus, top_k)
            .await
    }

    /// Process document with both LLM analysis and embeddings
    ///
    /// # Errors
    ///
    /// Returns an error if processing fails.
    pub async fn process_document(&self, content: &str) -> Result<DocumentEmbedding> {
        // Generate summary using LLM
        let summary = self.llm_client.summarize(content).await?;

        // Generate embeddings for both original and summary
        let content_embedding = self
            .embed_text_optimized(content, EmbeddingUseCase::SemanticSearch)
            .await?;
        let summary_embedding = self
            .embed_text_optimized(&summary, EmbeddingUseCase::SemanticSearch)
            .await?;

        Ok(DocumentEmbedding {
            content_embedding,
            summary_embedding,
            summary,
            token_count: self.estimate_tokens(content),
        })
    }

    /// Process multiple documents in batch
    ///
    /// # Errors
    ///
    /// Returns an error if batch processing fails.
    pub async fn process_documents_batch(
        &self,
        documents: Vec<DocumentInput>,
    ) -> Result<Vec<DocumentEmbedding>> {
        let mut results = Vec::with_capacity(documents.len());

        // Extract content for batch embedding
        let contents: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();

        // Generate embeddings in batch
        let content_embeddings = self.embed_texts(&contents).await?;

        // Generate summaries individually (LLM calls are harder to batch)
        for (i, doc) in documents.into_iter().enumerate() {
            let summary = self.llm_client.summarize(&doc.content).await?;
            let summary_embedding = self.embed_text(&summary).await?;

            results.push(DocumentEmbedding {
                content_embedding: content_embeddings[i].clone(),
                summary_embedding,
                summary,
                token_count: self.estimate_tokens(&doc.content),
            });
        }

        Ok(results)
    }

    /// Estimate token count for a text
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimation: 1 token ≈ 4 characters for English text
        (text.len() as f64 * 0.25).ceil() as usize
    }

    /// Get service statistics
    #[must_use]
    pub fn get_stats(&self) -> ServiceStats {
        ServiceStats {
            provider: self.llm_client.config().provider.clone(),
            has_batch_processor: self.batch_processor.is_some(),
            batch_stats: self.batch_processor.as_ref().map(|p| p.get_batch_stats()),
        }
    }
}

/// Input document for processing
#[derive(Debug, Clone)]
pub struct DocumentInput {
    /// Document content
    pub content: String,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Processed document with embeddings
#[derive(Debug, Clone)]
pub struct DocumentEmbedding {
    /// Embedding of the original content
    pub content_embedding: Vec<f32>,
    /// Embedding of the summary
    pub summary_embedding: Vec<f32>,
    /// Generated summary
    pub summary: String,
    /// Estimated token count
    pub token_count: usize,
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// LLM provider being used
    pub provider: LlmProvider,
    /// Whether batch processing is available
    pub has_batch_processor: bool,
    /// Batch processing statistics if available
    pub batch_stats: Option<BatchStats>,
}

impl DocumentEmbedding {
    /// Calculate similarity between this document and a query embedding
    #[must_use]
    pub fn similarity_to_query(&self, query_embedding: &[f32]) -> f32 {
        // Use content embedding for primary similarity
        LlmClient::cosine_similarity(&self.content_embedding, query_embedding)
    }

    /// Get combined similarity score (content + summary)
    #[must_use]
    pub fn combined_similarity(&self, query_embedding: &[f32]) -> f32 {
        let content_sim = LlmClient::cosine_similarity(&self.content_embedding, query_embedding);
        let summary_sim = LlmClient::cosine_similarity(&self.summary_embedding, query_embedding);

        // Weighted average: 70% content, 30% summary
        (content_sim * 0.7) + (summary_sim * 0.3)
    }
}

/// Adapter for integrating with external embedding clients
///
/// This provides a flexible way to integrate with external embedding systems
/// without creating circular dependencies.
pub struct ExternalEmbeddingAdapter<F> {
    /// Function that processes a batch of texts
    processor: F,
}

impl<F, Fut> ExternalEmbeddingAdapter<F>
where
    F: Fn(Vec<String>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<Vec<Vec<f32>>>> + Send,
{
    /// Create a new adapter with a custom processing function
    #[must_use]
    pub fn new(processor: F) -> Self {
        Self { processor }
    }
}

#[async_trait::async_trait]
impl<F, Fut> BatchEmbeddingProcessor for ExternalEmbeddingAdapter<F>
where
    F: Fn(Vec<String>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<Vec<Vec<f32>>>> + Send,
{
    async fn process_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        (self.processor)(texts).await
    }

    fn get_batch_stats(&self) -> BatchStats {
        // Return basic stats - external adapters would need to track their own stats
        BatchStats {
            total_processed: 0,
            successful: 0,
            failed: 0,
            avg_processing_time_ms: 0.0,
            cost_info: None,
        }
    }
}

/// Helper function to create an adapter for the embed crate
///
/// This can be used to integrate with the existing embed crate's capabilities
/// without creating circular dependencies.
#[must_use]
pub fn create_embed_crate_adapter() -> Option<Box<dyn BatchEmbeddingProcessor>> {
    // This would be implemented when integrating with the embed crate
    // For now, return None to indicate the adapter is not available
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_embedding_similarity() {
        let embedding = DocumentEmbedding {
            content_embedding: vec![1.0, 0.0, 0.0],
            summary_embedding: vec![0.0, 1.0, 0.0],
            summary: "test".to_string(),
            token_count: 10,
        };

        let query = vec![1.0, 0.0, 0.0];
        let similarity = embedding.similarity_to_query(&query);

        // Should be exactly 1.0 (perfect match)
        assert!((similarity - 1.0).abs() < f32::EPSILON);
    }
}
