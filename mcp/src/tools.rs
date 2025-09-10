//! MCP tool definitions

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use db::{
    models::ToolConfig,
    queries::{DocumentQueries, MetadataFilters},
    DatabasePool,
};
use embed::{EmbeddingClient, OpenAIEmbeddingClient};
use serde_json::{json, Value};
use sqlx::Row;
use std::fmt::Write as _;
use tracing::{debug, warn};

// Legacy IngestTool removed - use intelligent ingestion endpoint at /ingest/intelligent instead

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

        // Generate embeddings via OpenAI embedding client (Claude is not used here)
        let query_embedding = self.embedding_client.embed(query).await?;

        // Perform vector similarity search
        let results = DocumentQueries::rust_vector_search(
            self.db_pool.pool(),
            query,
            &query_embedding,
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
                doc.content.chars().take(1200).collect::<String>()
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

        // Use config doc_type directly (already in correct format)
        let db_doc_type = self.config.doc_type.as_str();

        // Handle discovery queries inline for Birdeye API
        if self.config.doc_type == "birdeye" {
            debug!("Birdeye query detected: '{}'", query);
            if self.is_discovery_query(query) {
                debug!("Discovery query triggered for: '{}'", query);
                return self.handle_birdeye_discovery_query(query, limit).await;
            } else {
                debug!("Regular search for Birdeye query: '{}'", query);
            }
        }

        // Try vector search first, fallback to text search if vector extension not available
        let results = match self
            .try_vector_search(query, db_doc_type, limit, filters.as_ref())
            .await
        {
            Ok(results) => results,
            Err(e) => {
                warn!("Vector search failed ({}), falling back to text search", e);
                self.text_search(query, db_doc_type, limit).await?
            }
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
            let relevance_score = Self::calculate_relevance_score(i, results.len());

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

    /// Check if query is a discovery request
    fn is_discovery_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Discovery patterns for Birdeye API
        if self.config.doc_type == "birdeye" {
            // Debug: Add some logging to understand what's happening
            debug!("Checking discovery query: '{}'", query_lower);

            let is_discovery = (query_lower.contains("list") && query_lower.contains("endpoint"))
                || query_lower.contains("what endpoints")
                || query_lower.contains("show me all")
                || query_lower.contains("available endpoints")
                || query_lower.contains("all price endpoints")
                || query_lower.contains("what price endpoints")
                || query_lower.contains("what apis")
                || query_lower.contains("what api")
                || query_lower.contains("api overview")
                || query_lower.contains("endpoint catalog");

            if is_discovery {
                debug!("Discovery query detected: '{}'", query_lower);
            }

            return is_discovery;
        }

        false
    }

    /// Handle Birdeye-specific discovery queries inline
    async fn handle_birdeye_discovery_query(&self, query: &str, limit: Option<i64>) -> Result<String> {
        let _query_lower = query.to_lowercase();
        let db_doc_type = "birdeye";
        let max_results = limit.unwrap_or(50) as usize;

        // Query for all endpoints with metadata
        let endpoints = sqlx::query(
            r"
            SELECT
                doc_path,
                metadata->>'method' as method,
                metadata->>'endpoint' as endpoint,
                metadata->>'api_version' as api_version,
                LEFT(content, 200) as preview
            FROM documents
            WHERE doc_type = $1
              AND doc_path LIKE '%/%'
            ORDER BY doc_path
            "
        )
        .bind(db_doc_type)
        .fetch_all(self.db_pool.pool())
        .await?;

        let mut response = String::from("# Birdeye API Endpoint Catalog\n\n");

        // Categorize endpoints
        let mut price_endpoints = Vec::new();
        let mut token_endpoints = Vec::new();
        let mut trade_endpoints = Vec::new();
        let mut wallet_endpoints = Vec::new();
        let mut other_endpoints = Vec::new();

        for row in &endpoints {
            let doc_path: String = row.get("doc_path");
            let method: Option<String> = row.get("method");
            let preview: Option<String> = row.get("preview");

            let endpoint_info = format!(
                "**{}** {}\n   {}",
                method.unwrap_or_else(|| "GET".to_string()),
                doc_path,
                preview.unwrap_or_else(|| "API endpoint documentation".to_string())
                    .chars().take(100).collect::<String>()
            );

            // Categorize based on path
            if doc_path.contains("/price") || doc_path.contains("/multi_price") {
                price_endpoints.push(endpoint_info);
            } else if doc_path.contains("/token") {
                token_endpoints.push(endpoint_info);
            } else if doc_path.contains("/txs") || doc_path.contains("/trader") {
                trade_endpoints.push(endpoint_info);
            } else if doc_path.contains("/wallet") {
                wallet_endpoints.push(endpoint_info);
            } else {
                other_endpoints.push(endpoint_info);
            }
        }

        // Add categorized sections
        if !price_endpoints.is_empty() {
            response.push_str(&format!("## Price Endpoints ({})\n\n", price_endpoints.len()));
            for endpoint in price_endpoints.into_iter().take(max_results) {
                response.push_str(&format!("• {}\n\n", endpoint));
            }
        }

        if !token_endpoints.is_empty() {
            response.push_str(&format!("## Token Endpoints ({})\n\n", token_endpoints.len()));
            for endpoint in token_endpoints.into_iter().take(max_results) {
                response.push_str(&format!("• {}\n\n", endpoint));
            }
        }

        if !trade_endpoints.is_empty() {
            response.push_str(&format!("## Trading Endpoints ({})\n\n", trade_endpoints.len()));
            for endpoint in trade_endpoints.into_iter().take(max_results) {
                response.push_str(&format!("• {}\n\n", endpoint));
            }
        }

        if !wallet_endpoints.is_empty() {
            response.push_str(&format!("## Wallet Endpoints ({})\n\n", wallet_endpoints.len()));
            for endpoint in wallet_endpoints.into_iter().take(max_results) {
                response.push_str(&format!("• {}\n\n", endpoint));
            }
        }

        if !other_endpoints.is_empty() {
            response.push_str(&format!("## Other Endpoints ({})\n\n", other_endpoints.len()));
            for endpoint in other_endpoints.into_iter().take(max_results) {
                response.push_str(&format!("• {}\n\n", endpoint));
            }
        }

        response.push_str(&format!("\n**Total Endpoints Available:** {}\n", endpoints.len()));
        response.push_str("\n💡 **Tip:** Use specific endpoint paths like `GET /defi/price` for detailed documentation.");

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
                    doc.content.chars().take(2000).collect::<String>(),
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
                    doc.content.chars().take(1000).collect::<String>()
                )
            }
            Some("markdown") | None => {
                // Format as markdown with proper structure
                let content_preview = doc.content.chars().take(1500).collect::<String>();
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
                    content_preview = doc.content.chars().take(1000).collect::<String>()
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

    /// Try vector search with real embeddings
    async fn try_vector_search(
        &self,
        query: &str,
        db_doc_type: &str,
        limit: Option<i64>,
        filters: Option<&MetadataFilters>,
    ) -> Result<Vec<db::models::Document>> {
        // Generate embeddings via OpenAI embedding client (Claude is not used here)
        let query_embedding = self.embedding_client.embed(query).await?;

        // Perform vector similarity search filtered by doc_type and metadata
        let results = if let Some(metadata_filters) = filters {
            DocumentQueries::doc_type_vector_search_with_filters(
                self.db_pool.pool(),
                db_doc_type,
                query,
                &query_embedding,
                limit.unwrap_or(5),
                metadata_filters,
            )
            .await?
        } else {
            DocumentQueries::doc_type_vector_search(
                self.db_pool.pool(),
                db_doc_type,
                query,
                &query_embedding,
                limit.unwrap_or(5),
            )
            .await?
        };

        Ok(results)
    }

    /// Fallback text search when vector search is not available
    async fn text_search(
        &self,
        query: &str,
        db_doc_type: &str,
        limit: Option<i64>,
    ) -> Result<Vec<db::models::Document>> {
        // Get all documents of this type and filter by query text
        let results = DocumentQueries::find_by_type_str(self.db_pool.pool(), db_doc_type).await?;

        // Filter results based on query text and rank by relevance
        let query_lower = query.to_lowercase();
        let mut filtered_results: Vec<_> = results
            .into_iter()
            .filter(|doc| {
                let content_match = doc.content.to_lowercase().contains(&query_lower);
                let path_match = doc.doc_path.to_lowercase().contains(&query_lower);
                content_match || path_match
            })
            .collect();

        // Sort by relevance (path matches first, then by content length)
        filtered_results.sort_by(|a, b| {
            let a_path_match = a.doc_path.to_lowercase().contains(&query_lower);
            let b_path_match = b.doc_path.to_lowercase().contains(&query_lower);

            match (a_path_match, b_path_match) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.content.len().cmp(&a.content.len()), // Longer content first
            }
        });

        // Take only the requested number of results
        filtered_results.truncate(usize::try_from(limit.unwrap_or(5)).unwrap_or(5));

        Ok(filtered_results)
    }

    /// Calculate a mock relevance score based on result position
    fn calculate_relevance_score(position: usize, _total: usize) -> f64 {
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

// Legacy IngestTool implementation removed - use intelligent ingestion endpoint instead

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
