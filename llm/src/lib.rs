//! LLM integration for summarization and query processing
//!
//! This crate provides integration with language models for summarizing
//! search results and processing user queries.

pub mod client;
pub mod models;
pub mod prompts;

pub use client::LlmClient;
// pub use models::*;  // Unused import
