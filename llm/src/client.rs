//! LLM client implementation

use anyhow::Result;

/// LLM client for summarization
#[derive(Default)]
pub struct LlmClient;

impl LlmClient {
    /// Create a new LLM client
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Summarize text
    ///
    /// # Errors
    ///
    /// This function currently returns `Ok` but in future may return an error if the
    /// underlying LLM API call fails.
    pub fn summarize(&self, _text: &str) -> Result<String> {
        // TODO: Implement LLM integration
        Ok("Summary placeholder".to_string())
    }
}
