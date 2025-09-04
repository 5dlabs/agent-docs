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
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;
use tracing::{debug, warn};

/// Server-side ingest tool that spawns the loader CLI
pub struct IngestTool;

impl Default for IngestTool {
    fn default() -> Self {
        Self::new()
    }
}

impl IngestTool {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    // Helper: run a command and capture combined output
    async fn run_cmd(cmd: &mut TokioCommand) -> anyhow::Result<String> {
        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let mut out = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).await.ok();
            out.push_str(&String::from_utf8_lossy(&buf));
        }
        if let Some(mut stderr) = child.stderr.take() {
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf).await.ok();
            out.push_str(&String::from_utf8_lossy(&buf));
        }
        let status = child.wait().await?;
        if !status.success() {
            return Err(anyhow::anyhow!(format!(
                "command failed: status={status}, output=\n{}",
                out
            )));
        }
        Ok(out)
    }

    fn sanitize_source_name(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    // Resolve loader binary path (env override with sensible default)
    fn loader_bin() -> PathBuf {
        std::env::var("LOADER_BIN").map_or_else(|_| PathBuf::from("/app/loader"), PathBuf::from)
    }

    // Safely resolve a subpath within the cloned repository, preventing path traversal
    async fn resolve_repo_subpath(repo_root: &Path, subpath: &str) -> Option<PathBuf> {
        let rel = Path::new(subpath);
        // Reject absolute paths and any parent directory components
        if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
            return None;
        }

        let candidate = repo_root.join(rel);
        // Canonicalize both repo_root and candidate to resolve symlinks and ensure containment
        let Ok(repo_canon) = tokio::fs::canonicalize(repo_root).await else {
            return None;
        };
        let Ok(cand_canon) = tokio::fs::canonicalize(&candidate).await else {
            return None;
        };

        if cand_canon.starts_with(&repo_canon) {
            Some(cand_canon)
        } else {
            None
        }
    }
}

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

        // Use our LLM implementation to generate real embeddings
        let llm_client = llm::LlmClient::new()?;
        let query_embedding = llm_client.generate_embedding(query).await?;

        // Perform vector similarity search
        let results = DocumentQueries::rust_vector_search(
            self.db_pool.pool(),
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

        // Use config doc_type directly (already in correct format)
        let db_doc_type = self.config.doc_type.as_str();

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

    /// Try vector search with real embeddings
    async fn try_vector_search(
        &self,
        query: &str,
        db_doc_type: &str,
        limit: Option<i64>,
        filters: Option<&MetadataFilters>,
    ) -> Result<Vec<db::models::Document>> {
        // Use our LLM implementation to generate real embeddings
        let llm_client = llm::LlmClient::new()?;
        let query_embedding = llm_client.generate_embedding(query).await?;

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

#[async_trait]
impl Tool for IngestTool {
    fn definition(&self) -> Value {
        json!({
            "name": "ingest",
            "description": "Ingest documentation by spawning the server-side loader CLI. Claude (or client) must provide repository URL, doc_type, paths, and extensions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repository_url": {"type": "string", "description": "Repository URL to clone (e.g., https://github.com/org/repo)"},
                    "doc_type": {"type": "string", "description": "Document type/category for storage (e.g., cilium, solana)"},
                    "paths": {"type": "string", "description": "Comma-separated include paths within the repo (e.g., docs/,README.md)"},
                    "extensions": {"type": "string", "description": "Comma-separated file extensions to include (e.g., md,rst,html)"},
                    "branch": {"type": "string", "description": "Optional branch/ref to checkout", "default": "HEAD"},
                    "source_name": {"type": "string", "description": "Optional source name for attribution (defaults to repo name)"},
                    "recursive": {"type": "boolean", "description": "Recursive directory traversal for local mode", "default": true}
                },
                "required": ["repository_url", "doc_type", "paths", "extensions"]
            }
        })
    }

    #[allow(clippy::too_many_lines)]
    async fn execute(&self, arguments: Value) -> anyhow::Result<String> {
        let repository_url = arguments
            .get("repository_url")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing required 'repository_url'"))?;
        let doc_type = arguments
            .get("doc_type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing required 'doc_type'"))?;
        let paths = arguments
            .get("paths")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing required 'paths' (comma-separated)"))?;
        let extensions = arguments
            .get("extensions")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing required 'extensions' (comma-separated)"))?;
        let branch = arguments
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or("HEAD");
        let recursive = arguments
            .get("recursive")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        // Derive a default source name from repo URL if not provided
        let default_source = repository_url
            .split('/')
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("/");
        let source_name = arguments
            .get("source_name")
            .and_then(Value::as_str)
            .map_or_else(
                || Self::sanitize_source_name(&default_source),
                Self::sanitize_source_name,
            );

        // Prepare temp directories
        let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let base = std::env::temp_dir().join(format!("ingest_{doc_type}_{ts}"));
        let repo_dir = base.join("repo");
        let out_dir = base.join("out");
        tokio::fs::create_dir_all(&repo_dir).await.ok();
        tokio::fs::create_dir_all(&out_dir).await.ok();

        // 1) Clone repository (shallow)
        let mut clone_cmd = TokioCommand::new("git");
        clone_cmd.args([
            "clone",
            "--depth",
            "1",
            repository_url,
            repo_dir.to_string_lossy().as_ref(),
        ]);
        let clone_out = Self::run_cmd(&mut clone_cmd).await?;

        // Optional checkout branch/ref
        if branch != "HEAD" {
            let mut co_cmd = TokioCommand::new("git");
            co_cmd.current_dir(&repo_dir).args(["checkout", branch]);
            Self::run_cmd(&mut co_cmd).await?;
        }

        // 2) Run loader local for each include path
        let mut processed_paths: Vec<String> = Vec::new();
        for raw in paths.split(',') {
            let p = raw.trim();
            if p.is_empty() {
                continue;
            }

            // Safely resolve subpath within repo root
            let Some(abs_path) = Self::resolve_repo_subpath(&repo_dir, p).await else {
                debug!("ingest: invalid or unsafe path, skipping: {}", p);
                continue;
            };

            let safe_dir = p.replace(['/', '\\'], "_");
            let path_out = out_dir.join(safe_dir);
            tokio::fs::create_dir_all(&path_out).await.ok();

            let rec_flag = if recursive { Some("--recursive") } else { None };

            let mut local_cmd = TokioCommand::new(Self::loader_bin());
            local_cmd
                .arg("local")
                .arg("--path")
                .arg(abs_path.to_string_lossy().as_ref())
                .arg("--extensions")
                .arg(extensions)
                .arg("-o")
                .arg(path_out.to_string_lossy().as_ref());
            if let Some(flag) = rec_flag {
                local_cmd.arg(flag);
            }

            let _local_out = Self::run_cmd(&mut local_cmd).await?;
            processed_paths.push(p.to_string());
        }

        // 3) Load into database
        let mut db_cmd = TokioCommand::new(Self::loader_bin());
        db_cmd
            .arg("database")
            .arg("--input-dir")
            .arg(out_dir.to_string_lossy().as_ref())
            .arg("--doc-type")
            .arg(doc_type)
            .arg("--source-name")
            .arg(&source_name)
            .arg("--yes");
        let db_out = Self::run_cmd(&mut db_cmd).await?;

        let mut resp = String::new();
        let _ = writeln!(resp, "✅ Ingestion completed");
        let _ = writeln!(resp, "Repository: {repository_url}");
        let _ = writeln!(resp, "Doc type: {doc_type}");
        let _ = writeln!(resp, "Source: {source_name}");
        let _ = writeln!(resp, "Paths processed: {}", processed_paths.join(", "));
        let _ = writeln!(resp, "Git output:\n{}", clone_out.trim());
        let _ = writeln!(resp, "DB load output:\n{}", db_out.trim());
        Ok(resp)
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
