//! Batch processing for embeddings

/// Batch processor for `OpenAI` API calls
#[derive(Default)]
pub struct BatchProcessor;

impl BatchProcessor {
    /// Create a new batch processor
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
