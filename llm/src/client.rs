//! LLM client implementation

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::models::{LlmProvider, LlmResponse, Message, ModelConfig, Usage};

/// `OpenAI` API request structure
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// `OpenAI` Embeddings API request structure
#[derive(Debug, Serialize)]
struct OpenAiEmbeddingRequest {
    input: String,
    model: String,
    encoding_format: String,
    dimensions: Option<u32>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// `OpenAI` API response structure
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

/// `OpenAI` Embeddings API response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiEmbeddingResponse {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: Option<OpenAiEmbeddingUsage>,
}

/// Embedding data from `OpenAI`
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: usize,
}

/// Usage information for embeddings
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiEmbeddingUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(clippy::struct_field_names)]
struct OpenAiUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

/// LLM client supporting both Claude Code and `OpenAI`
#[derive(Clone)]
pub struct LlmClient {
    config: ModelConfig,
    http_client: Client,
}

impl LlmClient {
    /// Create a new LLM client with default configuration (Claude Code)
    ///
    /// # Errors
    ///
    /// Returns an error if the Claude binary cannot be found or if required environment variables are missing.
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;
        Ok(Self {
            config,
            http_client: Client::new(),
        })
    }

    /// Create a new LLM client with custom configuration
    #[must_use]
    pub fn with_config(config: ModelConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if required configuration cannot be loaded.
    fn load_config() -> Result<ModelConfig> {
        // Try Claude Code first (default)
        if let Ok(binary_path) = env::var("CLAUDE_BINARY_PATH") {
            debug!(
                binary = %binary_path,
                "LLM config: using Claude binary from CLAUDE_BINARY_PATH"
            );
            return Ok(ModelConfig::claude_code(Some(binary_path)));
        }

        // Check if Claude binary exists in PATH
        if Self::binary_exists("claude") {
            debug!("LLM config: using Claude binary in PATH");
            return Ok(ModelConfig::claude_code(Some("claude".to_string())));
        }

        // Fall back to OpenAI if available
        if let Ok(api_key) = env::var("OPENAI_API_KEY") {
            if !api_key.trim().is_empty() {
                debug!("LLM config: using OpenAI provider (API key present)");
                return Ok(ModelConfig::openai(api_key));
            }
        }

        Err(anyhow!(
            "No LLM configuration found. Please set CLAUDE_BINARY_PATH or OPENAI_API_KEY environment variable."
        ))
    }

    /// Check if a binary exists in PATH
    fn binary_exists(name: &str) -> bool {
        std::process::Command::new("which")
            .arg(name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// Execute LLM request with the configured provider
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM execution fails.
    pub async fn execute(&self, messages: Vec<Message>) -> Result<LlmResponse> {
        debug!(
            provider = ?self.config.provider,
            model = %self.config.model_name,
            msg_count = messages.len(),
            "LLM execute invoked"
        );
        match self.config.provider {
            LlmProvider::ClaudeCode => self.execute_claude_code(messages).await,
            LlmProvider::OpenAI => self.execute_openai(messages).await,
        }
    }

    /// Execute using Claude Code binary
    async fn execute_claude_code(&self, messages: Vec<Message>) -> Result<LlmResponse> {
        let binary_path = self
            .config
            .binary_path
            .as_ref()
            .ok_or_else(|| anyhow!("Claude binary path not configured"))?;

        // Convert messages to Claude Code format
        let prompt = Self::format_messages_for_claude_code(messages);
        debug!(prompt_len = prompt.len(), "Prepared Claude prompt");

        // Execute Claude Code binary
        let output = self.run_claude_binary(binary_path, &prompt).await?;

        Ok(LlmResponse {
            content: output,
            usage: None, // Claude Code doesn't provide usage stats
            model: self.config.model_name.clone(),
        })
    }

    /// Execute using `OpenAI` API
    async fn execute_openai(&self, messages: Vec<Message>) -> Result<LlmResponse> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        // Convert messages to OpenAI format
        let openai_messages: Vec<OpenAiMessage> = messages
            .into_iter()
            .map(|msg| OpenAiMessage {
                role: msg.role,
                content: msg.content,
            })
            .collect();

        let request = OpenAiRequest {
            model: self.config.model_name.clone(),
            messages: openai_messages,
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
        };

        // Make the API request
        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let openai_response: OpenAiResponse = response.json().await?;
        let content = openai_response.choices.first().map_or_else(
            || "No response from `OpenAI`".to_string(),
            |choice| choice.message.content.clone(),
        );

        // Convert usage statistics
        let usage = openai_response.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(LlmResponse {
            content,
            usage,
            model: self.config.model_name.clone(),
        })
    }

    /// Run Claude Code binary with the given prompt using proper stream-json protocol
    #[allow(clippy::too_many_lines)]
    async fn run_claude_binary(&self, binary_path: &str, prompt: &str) -> Result<String> {
        let timeout_secs: u64 = std::env::var("CLAUDE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);

        let prompt_len = prompt.len();
        debug!(
            binary_path = %binary_path,
            model = %self.config.model_name,
            timeout_secs,
            prompt_len,
            "Launching Claude binary with stream-json protocol"
        );

        // Format the prompt as stream-json message

        // Build Claude command with proper arguments (matching CTO template)
        let mut cmd = Command::new(binary_path);
        cmd.arg("-p") // Prompt mode
            .arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("stream-json")
            .arg("--verbose")
            .env("CLAUDE_MODEL", &self.config.model_name)
            .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
            .env("DISABLE_TELEMETRY", "1")
            .env("DISABLE_ERROR_REPORTING", "1")
            .env("DISABLE_AUTOUPDATER", "1");

        // Allow additional CLI args via CLAUDE_ARGS (whitespace-separated)
        let mut extra_args_count = 0usize;
        if let Ok(extra_args) = env::var("CLAUDE_ARGS") {
            for arg in extra_args.split_whitespace() {
                cmd.arg(arg);
                extra_args_count += 1;
            }
        }

        let mut child = cmd
            .stdin(Stdio::piped()) // Use piped stdin for direct input
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start Claude binary '{}': {}", binary_path, e))?;

        #[allow(clippy::cast_sign_loss)]
        let pid = child.id().unwrap_or(0);
        info!("🚀 Claude process started (PID: {})", pid);
        debug!(
            binary = %binary_path,
            model = %self.config.model_name,
            extra_args = extra_args_count,
            pid,
            timeout_secs,
            "Claude process spawned with stream-json protocol"
        );

        // Format the prompt as stream-json message (matching CTO template)
        let json_message = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(prompt)?
        );

        info!("📤 Sending prompt to Claude ({} chars)", json_message.len());

        // Get stdin handle and write the JSON message directly
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin handle"))?;

        info!("📝 Writing JSON message to Claude stdin...");
        stdin
            .write_all(json_message.as_bytes())
            .await
            .map_err(|e| anyhow!("Failed to write to Claude stdin: {}", e))?;
        stdin
            .flush()
            .await
            .map_err(|e| anyhow!("Failed to flush Claude stdin: {}", e))?;
        drop(stdin); // Close stdin to signal end of input

        info!("✅ Successfully wrote to Claude stdin");

        // Wait for Claude to complete or timeout
        info!(
            "⏳ Waiting for Claude to complete (timeout: {}s)...",
            timeout_secs
        );
        let output_res = timeout(Duration::from_secs(timeout_secs), async {
            info!("🔄 Claude process finishing...");
            child.wait_with_output().await
        })
        .await;

        match output_res {
            Ok(Ok(out)) => {
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let stderr_snip = stderr.chars().take(1000).collect::<String>();
                    return Err(anyhow!(
                        "Claude binary exited with status: {}. stderr: {}",
                        out.status,
                        stderr_snip
                    ));
                }
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                // Enhanced logging for Claude output
                debug!(
                    exit_code = out.status.code().unwrap_or(-1),
                    stdout_len = stdout.len(),
                    stderr_len = stderr.len(),
                    "Claude completed with stream-json protocol"
                );

                // Log stdout if CLAUDE_LOG_STDOUT is set or if we're in verbose mode
                if std::env::var("CLAUDE_LOG_STDOUT").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    || std::env::var("CLAUDE_VERBOSE").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                {
                    let stdout_snip = stdout.chars().take(2000).collect::<String>();
                    info!("🔍 Claude stdout (truncated): {}", stdout_snip);
                }

                // Log stderr if CLAUDE_LOG_STDERR is set or if we're in verbose mode
                if std::env::var("CLAUDE_LOG_STDERR").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    || std::env::var("CLAUDE_VERBOSE").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                {
                    let stderr_snip = stderr.chars().take(2000).collect::<String>();
                    info!("🔍 Claude stderr (truncated): {}", stderr_snip);
                }

                // Always log Claude output in debug mode for troubleshooting
                tracing::debug!("Claude stdout (full): {}", stdout);
                tracing::debug!("Claude stderr (full): {}", stderr);
                if stdout.trim().is_empty() {
                    return Err(anyhow!("Claude binary returned empty response"));
                }
                Ok(stdout.trim().to_string())
            }
            Ok(Err(e)) => Err(anyhow!("Failed to wait for Claude process: {}", e)),
            Err(_) => Err(anyhow!(
                "Timeout reading from Claude binary (after {}s). Check ANTHROPIC_API_KEY, network egress, and consider increasing CLAUDE_TIMEOUT_SECS.",
                timeout_secs
            )),
        }
    }

    /// Format messages for Claude Code binary input
    fn format_messages_for_claude_code(messages: Vec<Message>) -> String {
        let mut prompt = String::new();

        for message in messages {
            match message.role.as_str() {
                "system" => {
                    prompt.push_str("System: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                "user" => {
                    prompt.push_str("Human: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                "assistant" => {
                    prompt.push_str("Assistant: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                _ => {}
            }
        }

        prompt.push_str("Assistant: ");
        prompt
    }

    /// Summarize text using the configured LLM
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM execution fails.
    pub async fn summarize(&self, text: &str) -> Result<String> {
        if text.len() > 50000 {
            // Token limit check
            return Ok(format!(
                "Text too long ({} chars), truncated analysis",
                text.len()
            ));
        }

        // Use the prompt factory to create a summarization prompt
        let prompt_builder = crate::prompts::PromptFactory::summarize_text(text);
        let messages = match self.config.provider {
            LlmProvider::ClaudeCode => vec![Message::user(prompt_builder.build_for_claude_code())],
            LlmProvider::OpenAI => prompt_builder.build_for_openai(),
        };

        let response = self.execute(messages).await?;
        Ok(response.content)
    }

    /// Analyze code using the configured LLM
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM execution fails.
    pub async fn analyze_code(&self, code: &str, language: &str) -> Result<String> {
        let prompt_builder = crate::prompts::PromptFactory::analyze_code(code, language, None);
        let messages = match self.config.provider {
            LlmProvider::ClaudeCode => vec![Message::user(prompt_builder.build_for_claude_code())],
            LlmProvider::OpenAI => prompt_builder.build_for_openai(),
        };

        let response = self.execute(messages).await?;
        Ok(response.content)
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &ModelConfig {
        &self.config
    }

    /// Generate embeddings for text using `OpenAI`
    ///
    /// # Errors
    ///
    /// Returns an error if the embedding generation fails.
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        match self.config.provider {
            LlmProvider::OpenAI => self.generate_openai_embedding(text).await,
            LlmProvider::ClaudeCode => {
                // Claude Code doesn't provide embeddings directly
                // Fall back to OpenAI or return an error
                Err(anyhow!(
                    "Claude Code does not support embeddings. Use OpenAI provider."
                ))
            }
        }
    }

    /// Generate embeddings using `OpenAI` Embeddings API
    async fn generate_openai_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| {
                anyhow!("OpenAI API key not configured. Set OPENAI_API_KEY environment variable.")
            })?;

        if text.trim().is_empty() {
            return Err(anyhow!("Cannot generate embedding for empty text"));
        }

        // Estimate token count (rough approximation)
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_sign_loss
        )]
        let estimated_tokens = (text.len() as f64 * 0.25) as u32;
        if estimated_tokens > 8191 {
            return Err(anyhow!(
                "Text too long for embedding (estimated {} tokens, max 8191)",
                estimated_tokens
            ));
        }

        let request = OpenAiEmbeddingRequest {
            input: text.to_string(),
            model: "text-embedding-3-large".to_string(),
            encoding_format: "float".to_string(),
            dimensions: Some(3072), // Full dimensionality
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI Embeddings API error: {}", error_text));
        }

        let embedding_response: OpenAiEmbeddingResponse = response.json().await?;

        if embedding_response.data.is_empty() {
            return Err(anyhow!("No embedding data received from OpenAI"));
        }

        let embedding = embedding_response.data[0].embedding.clone();

        debug!(
            "Generated embedding with {} dimensions for {} characters",
            embedding.len(),
            text.len()
        );

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts (batch)
    ///
    /// # Errors
    ///
    /// Returns an error if any embedding generation fails.
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.generate_embedding(text).await?;
            results.push(embedding);
        }

        Ok(results)
    }

    /// Calculate semantic similarity between two texts
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn calculate_similarity(&self, text1: &str, text2: &str) -> Result<f32> {
        let emb1 = self.generate_embedding(text1).await?;
        let emb2 = self.generate_embedding(text2).await?;

        if emb1.len() != emb2.len() {
            return Err(anyhow!("Embeddings have different dimensions"));
        }

        // Cosine similarity
        let dot_product: f32 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = emb2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm1 * norm2))
    }

    /// Find most similar texts from a corpus
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn find_most_similar(
        &self,
        query: &str,
        corpus: &[String],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        let query_embedding = self.generate_embedding(query).await?;
        let mut similarities = Vec::new();

        for text in corpus {
            let embedding = self.generate_embedding(text).await?;
            let similarity = Self::cosine_similarity(&query_embedding, &embedding);
            similarities.push((text.clone(), similarity));
        }

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k results
        Ok(similarities.into_iter().take(top_k).collect())
    }

    /// Calculate cosine similarity between two embeddings
    #[must_use]
    pub fn cosine_similarity(emb1: &[f32], emb2: &[f32]) -> f32 {
        if emb1.len() != emb2.len() {
            return 0.0;
        }

        let dot_product: f32 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = emb2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        dot_product / (norm1 * norm2)
    }

    /// Generate embeddings with optimized settings for specific use cases
    ///
    /// # Errors
    ///
    /// Returns an error if embedding generation fails.
    pub async fn generate_embedding_optimized(
        &self,
        text: &str,
        use_case: EmbeddingUseCase,
    ) -> Result<Vec<f32>> {
        match self.config.provider {
            LlmProvider::OpenAI => {
                self.generate_openai_embedding_optimized(text, use_case)
                    .await
            }
            LlmProvider::ClaudeCode => Err(anyhow!(
                "Claude Code does not support optimized embeddings. Use OpenAI provider."
            )),
        }
    }

    /// Generate optimized embeddings using `OpenAI`
    async fn generate_openai_embedding_optimized(
        &self,
        text: &str,
        use_case: EmbeddingUseCase,
    ) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| anyhow!("OpenAI API key not configured"))?;

        let (model, dimensions) = match use_case {
            EmbeddingUseCase::SemanticSearch => ("text-embedding-3-large", Some(3072)),
            EmbeddingUseCase::CodeSearch | EmbeddingUseCase::Classification => {
                ("text-embedding-3-small", Some(1536))
            }
            EmbeddingUseCase::Clustering => ("text-embedding-3-large", Some(1024)),
        };

        let request = OpenAiEmbeddingRequest {
            input: text.to_string(),
            model: model.to_string(),
            encoding_format: "float".to_string(),
            dimensions,
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI Embeddings API error: {}", error_text));
        }

        let embedding_response: OpenAiEmbeddingResponse = response.json().await?;

        if embedding_response.data.is_empty() {
            return Err(anyhow!("No embedding data received from OpenAI"));
        }

        Ok(embedding_response.data[0].embedding.clone())
    }
}

/// Use case-specific embedding optimization
#[derive(Debug, Clone, Copy)]
pub enum EmbeddingUseCase {
    /// General semantic search (high quality, full dimensions)
    SemanticSearch,
    /// Code search and analysis (balanced speed/quality)
    CodeSearch,
    /// Text classification tasks (balanced)
    Classification,
    /// Document clustering (reduced dimensions for efficiency)
    Clustering,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new().expect("Failed to create LLM client - check configuration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_code_formatting() {
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello"),
            Message::assistant("Hi there!"),
        ];

        let formatted = LlmClient::format_messages_for_claude_code(messages);
        assert!(formatted.contains("System:"));
        assert!(formatted.contains("Human:"));
        assert!(formatted.contains("Assistant:"));
        assert!(formatted.ends_with("Assistant: "));
    }
}
