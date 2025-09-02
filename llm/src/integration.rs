//! Integration examples for using LLM embeddings in document processing
//!
//! This module provides practical examples of how to integrate the enhanced
//! LLM client with document processing workflows, combining Claude Code
//! text generation with OpenAI embeddings.

use anyhow::Result;
use std::collections::HashMap;

use crate::client::{EmbeddingUseCase, LlmClient};
use crate::embeddings::EmbeddingService;
use crate::search::{HybridSearchEngine, SearchConfig};

/// Enhanced document processor that combines LLM analysis with embeddings
pub struct EnhancedDocumentProcessor {
    /// LLM client for text generation and analysis
    llm_client: LlmClient,
    /// Embedding service for vector operations
    embedding_service: EmbeddingService,
    /// Search engine for document retrieval
    search_engine: Option<HybridSearchEngine>,
}

impl EnhancedDocumentProcessor {
    /// Create a new enhanced document processor
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM client cannot be initialized.
    pub fn new() -> Result<Self> {
        let llm_client = LlmClient::new()?;
        let embedding_service = EmbeddingService::new(llm_client.clone());
        let search_engine = Some(HybridSearchEngine::new(embedding_service.clone()));

        Ok(Self {
            llm_client,
            embedding_service,
            search_engine,
        })
    }

    /// Process a document with full LLM and embedding analysis
    ///
    /// # Errors
    ///
    /// Returns an error if document processing fails.
    pub async fn process_document(&self, doc_id: String, content: String) -> Result<ProcessedDocument> {
        // Step 1: Use Claude Code to analyze and summarize the document
        let summary = self.llm_client.summarize(&content).await?;

        // Step 2: Generate embeddings for different use cases
        let semantic_embedding = self.embedding_service
            .embed_text_optimized(&content, EmbeddingUseCase::SemanticSearch)
            .await?;

        let code_embedding = if self.is_code_content(&content) {
            Some(self.embedding_service
                .embed_text_optimized(&content, EmbeddingUseCase::CodeSearch)
                .await?)
        } else {
            None
        };

        // Step 3: Extract keywords and metadata
        let keywords = self.extract_keywords(&content).await?;
        let metadata = self.extract_metadata(&content).await?;

        // Step 4: Calculate document quality score
        let quality_score = self.calculate_quality_score(&content, &summary).await?;

        // Step 5: Add to search index if available
        if let Some(ref search_engine) = self.search_engine {
            let search_engine = search_engine.clone(); // Clone for access
            // Note: In a real implementation, you'd want to modify the search engine
            // This is a simplified example
            let _ = search_engine;
        }

        Ok(ProcessedDocument {
            id: doc_id,
            content: content.clone(),
            summary,
            semantic_embedding,
            code_embedding,
            keywords,
            metadata,
            quality_score,
            token_count: self.embedding_service.estimate_tokens(&content),
        })
    }

    /// Process multiple documents in batch for efficiency
    ///
    /// # Errors
    ///
    /// Returns an error if batch processing fails.
    pub async fn process_documents_batch(&self, documents: Vec<(String, String)>) -> Result<Vec<ProcessedDocument>> {
        let mut processed_docs = Vec::new();

        // Extract content for batch embedding
        let contents: Vec<String> = documents.iter().map(|(_, content)| content.clone()).collect();

        // Generate embeddings in batch (more efficient)
        let semantic_embeddings = self.embedding_service.embed_texts(&contents).await?;

        // Process each document with LLM analysis
        for ((doc_id, content), semantic_embedding) in documents.into_iter().zip(semantic_embeddings) {
            // Generate summary using Claude Code
            let summary = self.llm_client.summarize(&content).await?;

            // Extract basic metadata
            let keywords = self.extract_keywords(&content).await?;
            let metadata = self.extract_metadata(&content).await?;
            let quality_score = self.calculate_quality_score(&content, &summary).await?;

            let processed_doc = ProcessedDocument {
                id: doc_id.clone(),
                content: content.clone(),
                summary,
                semantic_embedding,
                code_embedding: None, // Could be added with additional logic
                keywords,
                metadata,
                quality_score,
                token_count: self.embedding_service.estimate_tokens(&content),
            };

            processed_docs.push(processed_doc);
        }

        Ok(processed_docs)
    }

    /// Search documents using hybrid search
    ///
    /// # Errors
    ///
    /// Returns an error if search fails.
    pub async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::search::SearchResult>> {
        if let Some(ref search_engine) = self.search_engine {
            let config = SearchConfig::default();
            search_engine.search(query, limit, config.semantic_weight, config.keyword_weight).await
        } else {
            Err(anyhow::anyhow!("Search engine not available"))
        }
    }

    /// Generate embeddings for a query and find similar documents
    ///
    /// # Errors
    ///
    /// Returns an error if similarity search fails.
    pub async fn find_similar_documents(
        &self,
        query: &str,
        documents: &[ProcessedDocument],
        top_k: usize,
    ) -> Result<Vec<(ProcessedDocument, f32)>> {
        let query_embedding = self.embedding_service.embed_text(query).await?;

        let mut similarities: Vec<(ProcessedDocument, f32)> = documents
            .iter()
            .map(|doc| {
                let similarity = LlmClient::cosine_similarity(&query_embedding, &doc.semantic_embedding);
                (doc.clone(), similarity)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(similarities.into_iter().take(top_k).collect())
    }

    /// Classify document content type using LLM
    ///
    /// # Errors
    ///
    /// Returns an error if classification fails.
    pub async fn classify_document(&self, content: &str) -> Result<DocumentType> {
        let prompt = format!(
            "Classify the following document content into one of these categories: \
             technical_documentation, api_reference, tutorial, blog_post, research_paper, \
             code_example, configuration_file, other. \
             Return only the category name.\n\nContent:\n{}",
            &content[..content.len().min(1000)] // Limit content for prompt
        );

        let response = self.llm_client.summarize(&prompt).await?;
        let category = response.trim().to_lowercase();

        match category.as_str() {
            "technical_documentation" | "api_reference" => Ok(DocumentType::Technical),
            "tutorial" | "blog_post" => Ok(DocumentType::Educational),
            "research_paper" => Ok(DocumentType::Research),
            "code_example" => Ok(DocumentType::Code),
            "configuration_file" => Ok(DocumentType::Config),
            _ => Ok(DocumentType::Other),
        }
    }

    /// Extract keywords from document content
    async fn extract_keywords(&self, content: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Extract the 5-10 most important keywords or key phrases from the following text. \
             Return them as a comma-separated list.\n\nText:\n{}",
            &content[..content.len().min(2000)]
        );

        let response = self.llm_client.summarize(&prompt).await?;
        Ok(response
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Extract metadata from document
    async fn extract_metadata(&self, _content: &str) -> Result<HashMap<String, String>> {
        // This could be enhanced to extract more sophisticated metadata
        let mut metadata = HashMap::new();
        metadata.insert("processed_at".to_string(), chrono::Utc::now().to_rfc3339());
        metadata.insert("processor_version".to_string(), env!("CARGO_PKG_VERSION").to_string());
        Ok(metadata)
    }

    /// Calculate document quality score
    async fn calculate_quality_score(&self, content: &str, summary: &str) -> Result<f32> {
        // Simple quality scoring based on content length and summary coherence
        let content_length_score = (content.len() as f32 / 1000.0).min(1.0);
        let summary_length_score = (summary.len() as f32 / 200.0).min(1.0);

        // Could be enhanced with more sophisticated LLM-based quality assessment
        Ok((content_length_score + summary_length_score) / 2.0)
    }

    /// Check if content appears to be code
    fn is_code_content(&self, content: &str) -> bool {
        // Simple heuristic: check for code patterns
        let code_indicators = ["fn ", "struct ", "impl ", "use ", "mod ", "#[", "```", "let ", "const ", "pub ", "async ", "println!"];

        code_indicators.iter().any(|&indicator| content.contains(indicator))
    }

    /// Get processing statistics
    #[must_use]
    pub fn get_stats(&self) -> ProcessingStats {
        ProcessingStats {
            llm_provider: self.llm_client.config().provider.clone(),
            embedding_service_stats: self.embedding_service.get_stats(),
            search_enabled: self.search_engine.is_some(),
            search_stats: self.search_engine.as_ref().map(|se| se.get_stats()),
        }
    }
}

/// Processed document with full analysis
#[derive(Debug, Clone)]
pub struct ProcessedDocument {
    /// Document ID
    pub id: String,
    /// Original content
    pub content: String,
    /// LLM-generated summary
    pub summary: String,
    /// Semantic embedding for search
    pub semantic_embedding: Vec<f32>,
    /// Code-specific embedding (if applicable)
    pub code_embedding: Option<Vec<f32>>,
    /// Extracted keywords
    pub keywords: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Token count estimate
    pub token_count: usize,
}

/// Document type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentType {
    Technical,
    Educational,
    Research,
    Code,
    Config,
    Other,
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// LLM provider used
    pub llm_provider: crate::models::LlmProvider,
    /// Embedding service statistics
    pub embedding_service_stats: crate::embeddings::ServiceStats,
    /// Whether search is enabled
    pub search_enabled: bool,
    /// Search engine statistics
    pub search_stats: Option<crate::search::SearchStats>,
}

/// Example usage in a document processing pipeline
pub mod examples {
    use super::*;

    /// Example: Process GitHub repository documentation
    pub async fn process_github_repo_example() -> Result<()> {
        let processor = EnhancedDocumentProcessor::new()?;

        // Simulate processing README.md
        let readme_content = r#"
        # My Awesome Project

        This is a machine learning library for natural language processing.

        ## Features

        - Text classification
        - Named entity recognition
        - Sentiment analysis

        ## Installation

        ```bash
        cargo add my-awesome-project
        ```

        ## Usage

        ```rust
        use my_awesome_project::Classifier;

        let classifier = Classifier::new()?;
        let result = classifier.classify("This is amazing!")?;
        ```
        "#;

        let processed_doc = processor
            .process_document("readme".to_string(), readme_content.to_string())
            .await?;

        println!("Processed document: {}", processed_doc.id);
        println!("Summary: {}", processed_doc.summary);
        println!("Keywords: {:?}", processed_doc.keywords);
        println!("Quality score: {:.2}", processed_doc.quality_score);
        println!("Token count: {}", processed_doc.token_count);

        // Search example
        let search_results = processor.search_documents("machine learning", 5).await?;
        println!("Found {} search results", search_results.len());

        Ok(())
    }

    /// Example: Batch processing multiple documents
    pub async fn batch_processing_example() -> Result<()> {
        let processor = EnhancedDocumentProcessor::new()?;

        let documents = vec![
            ("doc1".to_string(), "This is about artificial intelligence and machine learning.".to_string()),
            ("doc2".to_string(), "This document covers natural language processing techniques.".to_string()),
            ("doc3".to_string(), "Here we discuss computer vision and image recognition.".to_string()),
        ];

        let processed_docs = processor.process_documents_batch(documents).await?;

        for doc in processed_docs {
            println!("Processed: {} - Quality: {:.2}", doc.id, doc.quality_score);
        }

        Ok(())
    }

    /// Example: Similarity search
    pub async fn similarity_search_example() -> Result<()> {
        let processor = EnhancedDocumentProcessor::new()?;

        // Create some sample documents
        let docs = vec![
            ProcessedDocument {
                id: "ai_doc".to_string(),
                content: "Artificial intelligence and machine learning".to_string(),
                summary: "AI and ML overview".to_string(),
                semantic_embedding: vec![0.1, 0.2, 0.3], // Would be real embeddings
                code_embedding: None,
                keywords: vec!["AI".to_string(), "ML".to_string()],
                metadata: HashMap::new(),
                quality_score: 0.8,
                token_count: 50,
            },
            ProcessedDocument {
                id: "nlp_doc".to_string(),
                content: "Natural language processing techniques".to_string(),
                summary: "NLP techniques".to_string(),
                semantic_embedding: vec![0.2, 0.3, 0.1], // Would be real embeddings
                code_embedding: None,
                keywords: vec!["NLP".to_string()],
                metadata: HashMap::new(),
                quality_score: 0.9,
                token_count: 40,
            },
        ];

        let similar_docs = processor
            .find_similar_documents("machine learning algorithms", &docs, 2)
            .await?;

        for (doc, similarity) in similar_docs {
            println!("Similar document: {} (score: {:.3})", doc.id, similarity);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_processing() {
        let processor = EnhancedDocumentProcessor::new().unwrap();

        let content = "This is a test document about machine learning and artificial intelligence.";
        let result = processor.process_document("test".to_string(), content.to_string()).await;

        // This test will only pass if OpenAI API key is configured
        // Otherwise it will fail gracefully with an error message
        match result {
            Ok(processed) => {
                assert_eq!(processed.id, "test");
                assert!(!processed.summary.is_empty());
                assert!(!processed.semantic_embedding.is_empty());
                assert!(processed.quality_score >= 0.0 && processed.quality_score <= 1.0);
            }
            Err(e) => {
                // Expected if API key not configured
                assert!(e.to_string().contains("API key") || e.to_string().contains("binary"));
            }
        }
    }

    #[test]
    fn test_document_type_classification() {
        let processor = EnhancedDocumentProcessor::new().unwrap();

        // Test code detection
        assert!(processor.is_code_content("fn main() { println!(\"Hello\"); }"));
        assert!(!processor.is_code_content("This is just plain text."));
    }
}
