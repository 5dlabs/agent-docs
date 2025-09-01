//! MCP tool definitions

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use db::{
    models::ToolConfig,
    queries::{DocumentQueries, MetadataFilters},
    DatabasePool,
};
use embed::OpenAIEmbeddingClient;
use serde_json::{json, Value};
use std::fmt::Write as _;
use tracing::debug;

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
                "{}. **{}** (from `{metadata}`)\n{}...\n\n",
                i + 1,
                doc.doc_path,
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

/// Dynamic query tool that works with any document type
pub struct DynamicQueryTool {
    config: ToolConfig,
    db_pool: DatabasePool,
    #[allow(dead_code)]
    embedding_client: OpenAIEmbeddingClient,
}

impl DynamicQueryTool {
    /// Create a new dynamic query tool
    ///
    /// # Errors
    ///
    /// Returns an error if the embedding client fails to initialize.
    pub fn new(config: ToolConfig, db_pool: DatabasePool) -> Result<Self> {
        let embedding_client = OpenAIEmbeddingClient::new()?;

        Ok(Self {
            config,
            db_pool,
            embedding_client,
        })
    }

    /// Perform semantic search for documents of the configured type
    async fn semantic_search(
        &self,
        query: &str,
        limit: Option<i64>,
        filters: Option<MetadataFilters>,
    ) -> Result<String> {
        debug!(
            "Performing {} documentation search for: {}",
            self.config.doc_type, query
        );

        // For now, use a simple database search (we'll add real embeddings later)
        let dummy_embedding = vec![0.0; 3072]; // Placeholder embedding

        // Use config doc_type directly (already in correct format)
        let db_doc_type = self.config.doc_type.as_str();

        // Perform vector similarity search filtered by doc_type and metadata
        let results = if let Some(metadata_filters) = filters {
            DocumentQueries::doc_type_vector_search_with_filters(
                self.db_pool.pool(),
                db_doc_type,
                &dummy_embedding,
                limit.unwrap_or(5),
                &metadata_filters,
            )
            .await?
        } else {
            DocumentQueries::doc_type_vector_search(
                self.db_pool.pool(),
                db_doc_type,
                &dummy_embedding,
                limit.unwrap_or(5),
            )
            .await?
        };

        if results.is_empty() {
            return Ok(format!(
                "No relevant {} documentation found for your query.",
                self.config.title
            ));
        }

        // Format results with source attribution and relevance
        let mut response = format!(
            "Found {} relevant {} results:\n\n",
            results.len(),
            self.config.title
        );

        for (i, doc) in results.iter().enumerate() {
            // Extract source information from metadata
            let source_info = self.extract_source_info(doc);
            let relevance_score = self.calculate_relevance_score(i, results.len());

            // Apply adaptive formatting based on content type
            let formatted_content = Self::format_content_adaptively(doc);

            let _ = write!(
                &mut response,
                "{}. **{}** ({source_info})\n*Relevance: {:.1}%*\n\n{formatted_content}\n\n",
                i + 1,
                doc.doc_path,
                relevance_score * 100.0,
            );
        }

        Ok(response)
    }

    /// Format content adaptively based on metadata hints
    fn format_content_adaptively(doc: &db::models::Document) -> String {
        // Check content format from metadata
        let format = doc
            .metadata
            .as_object()
            .and_then(|m| m.get("format"))
            .and_then(|f| f.as_str());

        match format {
            Some("bob" | "msc") => {
                // Preserve ASCII art structure for diagrams
                format!(
                    "```\n{}\n```\n\n*Diagram Content ({})*",
                    doc.content.chars().take(800).collect::<String>(),
                    format.unwrap_or("ascii")
                )
            }
            Some("pdf") => {
                // Show PDF metadata summary
                let size = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("size"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown size");
                let page_count = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("page_count"))
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);

                format!(
                    "**PDF Document Summary**\n\n- Size: {}\n- Pages: {}\n- Location: {}\n\n**Content Preview:**\n{}...",
                    size,
                    page_count,
                    doc.doc_path,
                    doc.content.chars().take(300).collect::<String>()
                )
            }
            Some("markdown") | None => {
                // Format as markdown with proper structure
                let content_preview = doc.content.chars().take(400).collect::<String>();
                if content_preview.starts_with('#') {
                    // Already has markdown headers
                    format!("{content_preview}...")
                } else {
                    // Add context
                    format!("```markdown\n{content_preview}...\n```")
                }
            }
            _ => {
                // Default formatting
                format!(
                    "{content_preview}...",
                    content_preview = doc.content.chars().take(300).collect::<String>()
                )
            }
        }
    }

    /// Extract source attribution from document metadata
    fn extract_source_info(&self, doc: &db::models::Document) -> String {
        match self.config.doc_type.as_str() {
            "rust" => {
                // Extract crate name from metadata
                doc.metadata
                    .as_object()
                    .and_then(|m| m.get("crate_name"))
                    .and_then(|c| c.as_str())
                    .map_or_else(
                        || format!("source: {}", doc.source_name),
                        |name| format!("from `{name}`"),
                    )
            }
            "birdeye" => {
                // Extract API endpoint and method info
                let endpoint = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("endpoint"))
                    .and_then(|e| e.as_str())
                    .unwrap_or("unknown endpoint");
                let method = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("method"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("GET");
                let api_version = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("api_version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("v1");
                format!("{method} {endpoint} - API {api_version}")
            }
            "solana" => {
                // Extract category and format info
                let category = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("category"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("docs");
                let format = doc
                    .metadata
                    .as_object()
                    .and_then(|m| m.get("format"))
                    .and_then(|f| f.as_str())
                    .unwrap_or("markdown");
                format!("{category} - {format}")
            }
            _ => {
                // Default source attribution
                format!("source: {}", doc.source_name)
            }
        }
    }

    /// Calculate a mock relevance score based on result position
    #[allow(clippy::unused_self)]
    fn calculate_relevance_score(&self, position: usize, _total: usize) -> f64 {
        // Simple declining relevance based on position
        // In practice, this would be based on actual vector similarity
        #[allow(clippy::cast_precision_loss)]
        {
            1.0 - (position as f64 * 0.1).min(0.5)
        }
    }
}

#[async_trait]
impl Tool for DynamicQueryTool {
    fn definition(&self) -> Value {
        let mut properties = json!({
            "query": {
                "type": "string",
                "description": "The search query. Can be a specific function name, concept, or natural language question."
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of results to return (default: 5, max: 20)",
                "minimum": 1,
                "maximum": 20
            }
        });

        // Add optional filter properties based on metadata hints
        if let Some(hints) = &self.config.metadata_hints {
            let properties_obj = properties.as_object_mut().unwrap();

            if !hints.supported_formats.is_empty() {
                properties_obj.insert("format".to_string(), json!({
                    "type": "string",
                    "description": format!("Filter by content format. Supported: {}", hints.supported_formats.join(", ")),
                    "enum": hints.supported_formats
                }));
            }

            if !hints.supported_complexity_levels.is_empty() {
                properties_obj.insert("complexity".to_string(), json!({
                    "type": "string", 
                    "description": format!("Filter by complexity level. Supported: {}", hints.supported_complexity_levels.join(", ")),
                    "enum": hints.supported_complexity_levels
                }));
            }

            if !hints.supported_categories.is_empty() {
                properties_obj.insert("category".to_string(), json!({
                    "type": "string",
                    "description": format!("Filter by category. Supported: {}", hints.supported_categories.join(", ")),
                    "enum": hints.supported_categories
                }));
            }

            if !hints.supported_topics.is_empty() {
                properties_obj.insert("topic".to_string(), json!({
                    "type": "string",
                    "description": format!("Filter by topic. Supported: {}", hints.supported_topics.join(", ")),
                    "enum": hints.supported_topics
                }));
            }

            if hints.supports_api_version {
                properties_obj.insert(
                    "api_version".to_string(),
                    json!({
                        "type": "string",
                        "description": "Filter by API version (e.g., 'v1', 'v2')"
                    }),
                );
            }
        }

        json!({
            "name": self.config.name,
            "description": self.config.description,
            "inputSchema": {
                "type": "object",
                "properties": properties,
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

        // Parse optional metadata filters
        let filters = self.parse_metadata_filters(&arguments)?;

        self.semantic_search(query, limit, filters).await
    }
}

impl DynamicQueryTool {
    /// Parse metadata filters from arguments
    fn parse_metadata_filters(&self, arguments: &Value) -> Result<Option<MetadataFilters>> {
        let mut filters = MetadataFilters::default();
        let mut has_filters = false;

        // Extract format filter
        if let Some(format_val) = arguments.get("format").and_then(|f| f.as_str()) {
            if let Some(hints) = &self.config.metadata_hints {
                if hints.supported_formats.contains(&format_val.to_string()) {
                    filters.format = Some(format_val.to_string());
                    has_filters = true;
                } else {
                    return Err(anyhow!(
                        "Unsupported format '{}'. Supported: {}",
                        format_val,
                        hints.supported_formats.join(", ")
                    ));
                }
            }
        }

        // Extract complexity filter
        if let Some(complexity_val) = arguments.get("complexity").and_then(|c| c.as_str()) {
            if let Some(hints) = &self.config.metadata_hints {
                if hints
                    .supported_complexity_levels
                    .contains(&complexity_val.to_string())
                {
                    filters.complexity = Some(complexity_val.to_string());
                    has_filters = true;
                } else {
                    return Err(anyhow!(
                        "Unsupported complexity '{}'. Supported: {}",
                        complexity_val,
                        hints.supported_complexity_levels.join(", ")
                    ));
                }
            }
        }

        // Extract category filter
        if let Some(category_val) = arguments.get("category").and_then(|c| c.as_str()) {
            if let Some(hints) = &self.config.metadata_hints {
                if hints
                    .supported_categories
                    .contains(&category_val.to_string())
                {
                    filters.category = Some(category_val.to_string());
                    has_filters = true;
                } else {
                    return Err(anyhow!(
                        "Unsupported category '{}'. Supported: {}",
                        category_val,
                        hints.supported_categories.join(", ")
                    ));
                }
            }
        }

        // Extract topic filter
        if let Some(topic_val) = arguments.get("topic").and_then(|t| t.as_str()) {
            if let Some(hints) = &self.config.metadata_hints {
                if hints.supported_topics.contains(&topic_val.to_string()) {
                    filters.topic = Some(topic_val.to_string());
                    has_filters = true;
                } else {
                    return Err(anyhow!(
                        "Unsupported topic '{}'. Supported: {}",
                        topic_val,
                        hints.supported_topics.join(", ")
                    ));
                }
            }
        }

        // Extract API version filter
        if let Some(api_version_val) = arguments.get("api_version").and_then(|v| v.as_str()) {
            if let Some(hints) = &self.config.metadata_hints {
                if hints.supports_api_version {
                    filters.api_version = Some(api_version_val.to_string());
                    has_filters = true;
                } else {
                    return Err(anyhow!("API version filtering not supported for this tool"));
                }
            }
        }

        if has_filters {
            Ok(Some(filters))
        } else {
            Ok(None)
        }
    }
}
