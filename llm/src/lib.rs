//! LLM integration for summarization and query processing
//!
//! This crate provides integration with language models for summarizing
//! search results and processing user queries. Supports both Claude Code
//! (local binary execution) and OpenAI API with embedding capabilities.

pub mod client;
pub mod embeddings;
pub mod integration;
pub mod models;
pub mod prompts;
pub mod search;

// Re-export main types
pub use client::{EmbeddingUseCase, LlmClient};
pub use embeddings::EmbeddingService;
pub use integration::EnhancedDocumentProcessor;
pub use models::{LlmProvider, LlmResponse, Message, ModelConfig, Usage};
pub use prompts::{PromptBuilder, PromptFactory, PromptTemplate};
pub use search::{HybridSearchEngine, SearchConfig, SearchResult, SearchStats};
