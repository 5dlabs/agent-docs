//! Hybrid search implementation combining semantic and keyword search
//!
//! This module provides hybrid search capabilities that combine:
//! 1. Semantic search using embeddings (vector similarity)
//! 2. Traditional keyword search (BM25-style)
//! 3. Combined ranking and filtering

use anyhow::Result;
use std::collections::{HashMap, HashSet};

use crate::embeddings::{DocumentEmbedding, EmbeddingService};

/// Search result with relevance scores
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Document ID
    pub document_id: String,
    /// Semantic similarity score (0.0 to 1.0)
    pub semantic_score: f32,
    /// Keyword relevance score (0.0 to 1.0)
    pub keyword_score: f32,
    /// Combined score using hybrid ranking
    pub combined_score: f32,
    /// Document content preview
    pub preview: String,
    /// Matched keywords
    pub matched_keywords: Vec<String>,
}

/// Hybrid search engine
#[derive(Clone)]
pub struct HybridSearchEngine {
    /// Embedding service for semantic search
    embedding_service: EmbeddingService,
    /// Document store with embeddings
    documents: HashMap<String, DocumentEmbedding>,
    /// Inverted index for keyword search
    inverted_index: HashMap<String, HashSet<String>>,
    /// Raw document content for keyword extraction
    document_content: HashMap<String, String>,
}

impl HybridSearchEngine {
    /// Create a new hybrid search engine
    #[must_use]
    pub fn new(embedding_service: EmbeddingService) -> Self {
        Self {
            embedding_service,
            documents: HashMap::new(),
            inverted_index: HashMap::new(),
            document_content: HashMap::new(),
        }
    }

    /// Add a document to the search index
    ///
    /// # Errors
    ///
    /// Returns an error if document processing fails.
    pub async fn add_document(&mut self, id: String, content: String) -> Result<()> {
        // Process document with embeddings
        let embedding = self.embedding_service.process_document(&content).await?;

        // Store document data
        self.documents.insert(id.clone(), embedding);
        self.document_content.insert(id.clone(), content.clone());

        // Build inverted index for keyword search
        self.build_inverted_index(&id, &content);

        Ok(())
    }

    /// Add multiple documents in batch
    ///
    /// # Errors
    ///
    /// Returns an error if batch processing fails.
    pub async fn add_documents_batch(&mut self, documents: Vec<(String, String)>) -> Result<()> {
        let contents: Vec<String> = documents
            .iter()
            .map(|(_, content)| content.clone())
            .collect();
        let embeddings = self.embedding_service.embed_texts(&contents).await?;

        for ((id, content), embedding) in documents.into_iter().zip(embeddings) {
            // Create document embedding (simplified - would normally include summary)
            let doc_embedding = DocumentEmbedding {
                content_embedding: embedding,
                summary_embedding: vec![], // Would generate summary embedding
                summary: String::new(),    // Would generate summary
                token_count: self.embedding_service.estimate_tokens(&content),
            };

            self.documents.insert(id.clone(), doc_embedding);
            self.document_content.insert(id.clone(), content.clone());
            self.build_inverted_index(&id, &content);
        }

        Ok(())
    }

    /// Perform hybrid search
    ///
    /// # Errors
    ///
    /// Returns an error if search fails.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        semantic_weight: f32,
        keyword_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        // Generate query embedding for semantic search
        let query_embedding = self.embedding_service.embed_text(query).await?;

        // Extract keywords from query for keyword search
        let query_keywords = Self::extract_keywords(query);

        // Perform semantic search
        let semantic_results = self.semantic_search(&query_embedding, limit * 2);

        // Perform keyword search
        let keyword_results = self.keyword_search(&query_keywords, limit * 2);

        // Combine results using hybrid ranking
        let combined_results = Self::combine_results(
            semantic_results,
            keyword_results,
            semantic_weight,
            keyword_weight,
            limit,
        );

        // Format final results
        let mut results = Vec::new();
        for (doc_id, semantic_score, keyword_score, combined_score) in combined_results {
            let matched_keywords = self.find_matched_keywords(&doc_id, &query_keywords);
            let preview = self.generate_preview(&doc_id, query);

            results.push(SearchResult {
                document_id: doc_id,
                semantic_score,
                keyword_score,
                combined_score,
                preview,
                matched_keywords,
            });
        }

        Ok(results)
    }

    /// Perform semantic search using embeddings
    fn semantic_search(&self, query_embedding: &[f32], limit: usize) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = self
            .documents
            .iter()
            .filter_map(|(id, doc)| {
                let similarity = doc.similarity_to_query(query_embedding);
                if similarity > 0.0 {
                    Some((id.clone(), similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.into_iter().take(limit).collect()
    }

    /// Perform keyword search using inverted index
    fn keyword_search(&self, keywords: &[String], limit: usize) -> Vec<(String, f32)> {
        if keywords.is_empty() {
            return Vec::new();
        }

        let mut doc_scores: HashMap<String, f32> = HashMap::new();

        // Calculate BM25-style scores for each document
        for keyword in keywords {
            if let Some(doc_ids) = self.inverted_index.get(keyword) {
                #[allow(clippy::cast_precision_loss)]
                let df = doc_ids.len() as f32; // Document frequency
                #[allow(clippy::cast_precision_loss)]
                let n = self.documents.len() as f32; // Total documents
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln(); // IDF

                for doc_id in doc_ids {
                    let tf = self.term_frequency(doc_id, keyword); // Term frequency
                    let score = idf * (tf * (1.0 + 1.0)) / (tf + 1.0); // BM25-like scoring

                    *doc_scores.entry(doc_id.clone()).or_insert(0.0) += score;
                }
            }
        }

        // Convert to vector and sort
        let mut results: Vec<(String, f32)> = doc_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        results.into_iter().take(limit).collect()
    }

    /// Combine semantic and keyword search results
    fn combine_results(
        semantic_results: Vec<(String, f32)>,
        keyword_results: Vec<(String, f32)>,
        semantic_weight: f32,
        keyword_weight: f32,
        limit: usize,
    ) -> Vec<(String, f32, f32, f32)> {
        let mut combined_scores: HashMap<String, (f32, f32, f32)> = HashMap::new();

        // Add semantic results
        for (doc_id, semantic_score) in semantic_results {
            combined_scores.insert(doc_id, (semantic_score, 0.0, 0.0));
        }

        // Add/merge keyword results
        for (doc_id, keyword_score) in keyword_results {
            let (semantic_score, _, _) = combined_scores.get(&doc_id).unwrap_or(&(0.0, 0.0, 0.0));
            combined_scores.insert(doc_id, (*semantic_score, keyword_score, 0.0));
        }

        // Calculate combined scores
        let mut results: Vec<(String, f32, f32, f32)> = combined_scores
            .into_iter()
            .map(|(doc_id, (semantic_score, keyword_score, _))| {
                let combined_score =
                    semantic_weight * semantic_score + keyword_weight * keyword_score;
                (doc_id, semantic_score, keyword_score, combined_score)
            })
            .collect();

        // Sort by combined score (descending)
        results.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        results.into_iter().take(limit).collect()
    }

    /// Extract keywords from query text
    fn extract_keywords(query: &str) -> Vec<String> {
        query
            .to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 2) // Filter out short words
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|word| !word.is_empty())
            .map(std::string::ToString::to_string)
            .collect()
    }

    /// Build inverted index for a document
    fn build_inverted_index(&mut self, doc_id: &str, content: &str) {
        let keywords = Self::extract_keywords(content);

        for keyword in keywords {
            self.inverted_index
                .entry(keyword)
                .or_default()
                .insert(doc_id.to_string());
        }
    }

    /// Calculate term frequency for a document and keyword
    fn term_frequency(&self, doc_id: &str, keyword: &str) -> f32 {
        if let Some(content) = self.document_content.get(doc_id) {
            #[allow(clippy::cast_precision_loss)]
            let total_words = content.split_whitespace().count() as f32;
            #[allow(clippy::cast_precision_loss)]
            let keyword_count = content
                .to_lowercase()
                .matches(&keyword.to_lowercase())
                .count() as f32;

            if total_words > 0.0 {
                keyword_count / total_words
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Find keywords that matched in a document
    fn find_matched_keywords(&self, doc_id: &str, query_keywords: &[String]) -> Vec<String> {
        if let Some(content) = self.document_content.get(doc_id) {
            let content_lower = content.to_lowercase();
            query_keywords
                .iter()
                .filter(|keyword| content_lower.contains(&keyword.to_lowercase()))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Generate preview text around matched content
    fn generate_preview(&self, doc_id: &str, query: &str) -> String {
        if let Some(content) = self.document_content.get(doc_id) {
            // Find first occurrence of query terms
            let query_lower = query.to_lowercase();
            if let Some(pos) = content.to_lowercase().find(&query_lower) {
                let start = pos.saturating_sub(50);
                let end = (pos + query.len() + 100).min(content.len());

                let preview = &content[start..end];
                format!("...{}...", preview.trim())
            } else {
                // Fallback to first 200 characters
                format!("{}...", &content[..content.len().min(200)])
            }
        } else {
            "Preview not available".to_string()
        }
    }

    /// Get search statistics
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn get_stats(&self) -> SearchStats {
        SearchStats {
            total_documents: self.documents.len(),
            total_keywords: self.inverted_index.len(),
            avg_document_length: if self.documents.is_empty() {
                0.0
            } else {
                self.documents
                    .values()
                    .map(|d| d.token_count as f64)
                    .sum::<f64>()
                    / self.documents.len() as f64
            },
        }
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) {
        self.documents.remove(doc_id);
        self.document_content.remove(doc_id);

        // Remove from inverted index
        self.inverted_index.retain(|_, doc_ids| {
            doc_ids.remove(doc_id);
            !doc_ids.is_empty()
        });
    }

    /// Clear all documents and rebuild index
    pub fn clear(&mut self) {
        self.documents.clear();
        self.inverted_index.clear();
        self.document_content.clear();
    }
}

/// Search statistics
#[derive(Debug, Clone)]
pub struct SearchStats {
    /// Total number of indexed documents
    pub total_documents: usize,
    /// Total number of unique keywords in index
    pub total_keywords: usize,
    /// Average document length in tokens
    pub avg_document_length: f64,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Weight for semantic similarity (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Weight for keyword relevance (0.0 to 1.0)
    pub keyword_weight: f32,
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum similarity threshold for semantic search
    pub min_similarity: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.3,
            max_results: 10,
            min_similarity: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::LlmClient;

    #[tokio::test]
    async fn test_hybrid_search_basic() {
        let Ok(llm_client) = LlmClient::new() else {
            eprintln!("Skipping test_hybrid_search_basic - no LLM config");
            return;
        };
        let embedding_service = EmbeddingService::new(llm_client);
        let mut search_engine = HybridSearchEngine::new(embedding_service);

        // Add some test documents
        search_engine
            .add_document(
                "doc1".to_string(),
                "This is a test document about machine learning".to_string(),
            )
            .await
            .unwrap();
        search_engine
            .add_document(
                "doc2".to_string(),
                "Another document discussing artificial intelligence".to_string(),
            )
            .await
            .unwrap();

        // Perform search
        let results = search_engine
            .search("machine learning", 5, 0.7, 0.3)
            .await
            .unwrap();

        // Should find results
        assert!(!results.is_empty());
    }

    #[test]
    fn test_keyword_extraction() {
        let keywords = HybridSearchEngine::extract_keywords("What is machine learning?");
        assert!(keywords.contains(&"machine".to_string()));
        assert!(keywords.contains(&"learning".to_string()));
    }
}
