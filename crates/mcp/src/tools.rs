//! MCP tool definitions

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use doc_server_database::{queries::DocumentQueries, DatabasePool};
use doc_server_embeddings::OpenAIEmbeddingClient;
use serde_json::{json, Value};
use tracing::debug;
use std::fmt::Write as _;

/// Base trait for MCP tools
#[async_trait]
pub trait Tool {
    /// Get the tool definition for MCP
    fn definition(&self) -> Value;

    /// Execute the tool with given arguments
    async fn execute(&self, arguments: Value) -> Result<String>;
}

/// Rust documentation query tool
pub struct RustQueryTool {
    db_pool: DatabasePool,
    #[allow(dead_code)]
    embedding_client: OpenAIEmbeddingClient,
}

impl RustQueryTool {
    /// Create a new Rust query tool
    /// Create a new Rust query tool.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedding client fails to initialize.
    pub fn new(db_pool: DatabasePool) -> Result<Self> {
        let embedding_client = OpenAIEmbeddingClient::new()?;

        Ok(Self {
            db_pool,
            embedding_client,
        })
    }

    /// Perform semantic search for Rust documentation
    async fn semantic_search(&self, query: &str, limit: Option<i64>) -> Result<String> {
        debug!("Performing Rust documentation search for: {}", query);

        // For now, use a simple database search (we'll add real embeddings later)
        let dummy_embedding = vec![0.0; 3072]; // Placeholder embedding

        // Perform vector similarity search
        let results = DocumentQueries::rust_vector_search(
            self.db_pool.pool(),
            &dummy_embedding,
            limit.unwrap_or(5),
        )
        .await?;

        if results.is_empty() {
            return Ok("No relevant Rust documentation found for your query.".to_string());
        }

        // Format results
        let mut response = format!(
            "Found {} relevant Rust documentation results:\n\n",
            results.len()
        );

        for (i, doc) in results.iter().enumerate() {
            let metadata = doc
                .metadata
                .as_object()
                .and_then(|m| m.get("crate_name"))
                .and_then(|c| c.as_str())
                .unwrap_or("unknown");

            let _ = write!(
                &mut response,
                "{}. **{}** (from `{}`)\n{}...\n\n",
                i + 1,
                doc.doc_path,
                metadata,
                doc.content.chars().take(300).collect::<String>()
            );
        }

        Ok(response)
    }
}

#[async_trait]
impl Tool for RustQueryTool {
    fn definition(&self) -> Value {
        json!({
            "name": "rust_query",
            "description": "Search and retrieve information from Rust crate documentation. Query across 40+ popular Rust crates including tokio, serde, clap, sqlx, axum, and more.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query. Can be a specific function name, concept, or natural language question about Rust code."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 5, max: 20)",
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["query"]
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let query = arguments
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| anyhow!("Missing required 'query' parameter"))?;

        let limit = arguments.get("limit").and_then(Value::as_i64);

        // Validate limit
        if let Some(l) = limit {
            if !(1..=20).contains(&l) {
                return Err(anyhow!("Limit must be between 1 and 20"));
            }
        }

        self.semantic_search(query, limit).await
    }
}
