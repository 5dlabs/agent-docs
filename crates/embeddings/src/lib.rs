//! Embedding generation and processing
//! 
//! This crate handles OpenAI API integration for generating embeddings,
//! batch processing for cost optimization, and vector operations.

pub mod client;
pub mod batch;
pub mod models;

pub use client::EmbeddingClient;
pub use batch::BatchProcessor;
pub use models::*;

/// Re-export pgvector types
pub use pgvector::Vector;