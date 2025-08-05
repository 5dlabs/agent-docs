# Project: docs

## Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/database",
    "crates/mcp", 
    "crates/embeddings",
    "crates/doc-loader",
    "crates/llm",
]

# Workspace-level dependencies that can be inherited by member crates
[workspace.dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
async-stream = "0.3"
futures = "0.3"

# Database and ORM
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP and web
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
hyper = "1.5"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Configuration and environment
dotenvy = "0.15"
config = "0.14"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# UUID generation
uuid = { version = "1.11", features = ["v4", "serde"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# MCP protocol implementation
rmcp = "0.1"

# Vector operations (for pgvector compatibility)
pgvector = { version = "0.4", features = ["serde"] }

# Testing
mockall = "0.13"
tokio-test = "0.4"
assert_matches = "1.5"

# Development tools dependencies
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"

# Allow some pedantic lints that are too noisy
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
```

## Source Files

### Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "crates/database",
    "crates/mcp", 
    "crates/embeddings",
    "crates/doc-loader",
    "crates/llm",
]

# Workspace-level dependencies that can be inherited by member crates
[workspace.dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
async-stream = "0.3"
futures = "0.3"

# Database and ORM
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# HTTP and web
axum = "0.7"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
hyper = "1.5"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Configuration and environment
dotenvy = "0.15"
config = "0.14"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# UUID generation
uuid = { version = "1.11", features = ["v4", "serde"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# MCP protocol implementation
rmcp = "0.1"

# Vector operations (for pgvector compatibility)
pgvector = { version = "0.4", features = ["serde"] }

# Testing
mockall = "0.13"
tokio-test = "0.4"
assert_matches = "1.5"

# Development tools dependencies
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"

# Allow some pedantic lints that are too noisy
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
```

### crates/database/Cargo.toml

```toml
[package]
name = "doc-server-database"
version = "0.1.0"
edition = "2021"
description = "Database layer for the Doc Server with PostgreSQL and pgvector support"
license = "MIT"

[dependencies]
# Inherit workspace dependencies
tokio = { workspace = true }
sqlx = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
dotenvy = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
pgvector = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
```

### crates/database/src/lib.rs

```rust
//! Database layer for the Doc Server
//! 
//! This crate provides database connection, schema management, and query operations
//! for the Doc Server using PostgreSQL with pgvector extension.

pub mod connection;
pub mod models;
pub mod queries;
pub mod migrations;

pub use connection::DatabasePool;
pub use models::*;

/// Re-export commonly used types
pub use sqlx::{PgPool, Row};
pub use uuid::Uuid;
```

### crates/database/src/models.rs

```rust
//! Database models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Document types supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "doc_type", rename_all = "snake_case")]
pub enum DocType {
    Rust,
    Jupyter,
    Birdeye,
    Cilium,
    Talos,
    Meteora,
    Raydium,
    Solana,
    Ebpf,
    RustBestPractices,
}

/// Main document record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub doc_type: String, // We'll handle the enum conversion manually
    pub source_name: String,
    pub doc_path: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Option<pgvector::Vector>,
    pub token_count: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Document source configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DocumentSource {
    pub id: Uuid,
    pub doc_type: DocType,
    pub source_name: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### crates/database/src/queries.rs

```rust
//! Database query operations

use anyhow::Result;
use sqlx::{PgPool, Row};

use crate::models::{Document, DocType};

/// Document query operations
pub struct DocumentQueries;

impl DocumentQueries {
    /// Find documents by type
    pub async fn find_by_type(pool: &PgPool, doc_type: DocType) -> Result<Vec<Document>> {
        let type_str = match doc_type {
            DocType::Rust => "rust",
            DocType::Jupyter => "jupyter",
            DocType::Birdeye => "birdeye",
            DocType::Cilium => "cilium",
            DocType::Talos => "talos",
            DocType::Meteora => "meteora",
            DocType::Raydium => "raydium",
            DocType::Solana => "solana",
            DocType::Ebpf => "ebpf",
            DocType::RustBestPractices => "rust_best_practices",
        };
        
        let rows = sqlx::query(
            r#"
            SELECT 
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            FROM documents 
            WHERE doc_type::text = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(type_str)
        .fetch_all(pool)
        .await?;
        
        let docs = rows.into_iter().map(|row| {
            Document {
                id: row.get("id"),
                doc_type: row.get("doc_type"),
                source_name: row.get("source_name"),
                doc_path: row.get("doc_path"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                embedding: None, // Skip embedding for now
                token_count: row.get("token_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }).collect();
        
        Ok(docs)
    }
    
    /// Find documents by source name
    pub async fn find_by_source(pool: &PgPool, source_name: &str) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            FROM documents 
            WHERE source_name = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(source_name)
        .fetch_all(pool)
        .await?;
        
        let docs = rows.into_iter().map(|row| {
            Document {
                id: row.get("id"),
                doc_type: row.get("doc_type"),
                source_name: row.get("source_name"),
                doc_path: row.get("doc_path"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                embedding: None, // Skip embedding for now
                token_count: row.get("token_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }).collect();
        
        Ok(docs)
    }
    
    /// Perform vector similarity search
    pub async fn vector_search(
        pool: &PgPool, 
        _embedding: &[f32], 
        limit: i64
    ) -> Result<Vec<Document>> {
        // For now, return a basic text search as fallback
        let rows = sqlx::query(
            r#"
            SELECT 
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            FROM documents 
            WHERE content IS NOT NULL
            ORDER BY created_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        
        let docs = rows.into_iter().map(|row| {
            Document {
                id: row.get("id"),
                doc_type: row.get("doc_type"),
                source_name: row.get("source_name"),
                doc_path: row.get("doc_path"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                embedding: None, // Skip embedding for now
                token_count: row.get("token_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }).collect();
        
        Ok(docs)
    }
    
    /// Perform vector similarity search for Rust documents only
    pub async fn rust_vector_search(
        pool: &PgPool,
        _embedding: &[f32],
        limit: i64
    ) -> Result<Vec<Document>> {
        // For now, return Rust documents ordered by relevance
        let rows = sqlx::query(
            r#"
            SELECT 
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            FROM documents 
            WHERE doc_type = 'rust'
            ORDER BY created_at DESC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        
        let docs = rows.into_iter().map(|row| {
            Document {
                id: row.get("id"),
                doc_type: row.get("doc_type"),
                source_name: row.get("source_name"),
                doc_path: row.get("doc_path"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                embedding: None, // Skip embedding for now
                token_count: row.get("token_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }).collect();
        
        Ok(docs)
    }
}
```

### crates/database/src/migrations.rs

```rust
//! Database migrations and schema management

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

/// Database migration operations
pub struct Migrations;

impl Migrations {
    /// Run all pending migrations
    pub async fn run(pool: &PgPool) -> Result<()> {
        info!("Running database migrations...");
        
        // Create extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(pool)
            .await?;
            
        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
            .execute(pool)
            .await?;
        
        // Create enum types
        sqlx::query(r#"
            DO $$ BEGIN
                CREATE TYPE doc_type AS ENUM (
                    'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
                    'meteora', 'raydium', 'solana', 'ebpf', 'rust_best_practices'
                );
            EXCEPTION
                WHEN duplicate_object THEN null;
            END $$;
        "#)
        .execute(pool)
        .await?;
        
        // Create documents table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS documents (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                doc_type doc_type NOT NULL,
                source_name VARCHAR(255) NOT NULL,
                doc_path TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata JSONB DEFAULT '{}',
                embedding vector(3072),
                token_count INTEGER,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(doc_type, source_name, doc_path)
            )
        "#)
        .execute(pool)
        .await?;
        
        // Create document_sources table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS document_sources (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                doc_type doc_type NOT NULL,
                source_name VARCHAR(255) NOT NULL,
                config JSONB DEFAULT '{}',
                enabled BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(doc_type, source_name)
            )
        "#)
        .execute(pool)
        .await?;
        
        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type)")
            .execute(pool)
            .await?;
            
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name)")
            .execute(pool)
            .await?;
            
        // Note: Skipping vector index for 3072-dimensional embeddings due to pgvector 2000-dimension limit
        // Queries will still work but be slower. Consider upgrading pgvector or using 1536 dimensions if performance is critical.
        
        info!("Database migrations completed successfully");
        Ok(())
    }
}
```

### crates/database/src/connection.rs

```rust
//! Database connection management

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database pool from connection URL
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database...");
        
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;
        
        info!("Database connection established");
        
        Ok(Self { pool })
    }
    
    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    /// Test the database connection
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
```

### crates/doc-loader/Cargo.toml

```toml
[package]
name = "doc-server-doc-loader"
version = "0.1.0"
edition = "2021"
description = "Document loading and parsing for various documentation types"
license = "MIT"

[dependencies]
# Inherit workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
reqwest = { workspace = true }

# HTML parsing for docs.rs content
scraper = "0.20"
html5ever = "0.27"

# URL handling
url = "2.5"

[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
```

### crates/doc-loader/src/parsers.rs

```rust
//! Document parsers

// TODO: Implement document parsing logic
```

### crates/doc-loader/src/loaders.rs

```rust
//! Document loaders for various sources

/// Rust crate documentation loader
pub struct RustLoader;

impl RustLoader {
    /// Create a new Rust loader
    pub fn new() -> Self {
        Self
    }
}
```

### crates/doc-loader/src/lib.rs

```rust
//! Document loading and parsing
//! 
//! This crate provides document loading functionality for various documentation
//! types including Rust crates, Jupyter notebooks, and API documentation.

pub mod loaders;
pub mod parsers;
pub mod extractors;

pub use loaders::*;

/// Re-export commonly used types
pub use url::Url;
```

### crates/doc-loader/src/extractors.rs

```rust
//! Content extractors

// TODO: Implement content extraction logic
```

### crates/llm/Cargo.toml

```toml
[package]
name = "doc-server-llm"
version = "0.1.0"
edition = "2021"
description = "LLM integration for summarization and query processing"
license = "MIT"

[dependencies]
# Inherit workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
reqwest = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
```

### crates/llm/src/prompts.rs

```rust
//! LLM prompts and templates

// TODO: Implement prompt templates
```

### crates/llm/src/client.rs

```rust
//! LLM client implementation

use anyhow::Result;

/// LLM client for summarization
pub struct LlmClient;

impl LlmClient {
    /// Create a new LLM client
    pub fn new() -> Self {
        Self
    }
    
    /// Summarize text
    pub async fn summarize(&self, _text: &str) -> Result<String> {
        // TODO: Implement LLM integration
        Ok("Summary placeholder".to_string())
    }
}
```

### crates/llm/src/lib.rs

```rust
//! LLM integration for summarization and query processing
//! 
//! This crate provides integration with language models for summarizing
//! search results and processing user queries.

pub mod client;
pub mod models;
pub mod prompts;

pub use client::LlmClient;
pub use models::*;
```

### crates/llm/src/models.rs

```rust
//! LLM models and types

// TODO: Implement LLM model types
```

### crates/embeddings/Cargo.toml

```toml
[package]
name = "doc-server-embeddings"
version = "0.1.0"
edition = "2021"
description = "Embedding generation and processing for the Doc Server using OpenAI API"
license = "MIT"

[dependencies]
# Inherit workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
reqwest = { workspace = true }
pgvector = { workspace = true }

[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
```

### crates/embeddings/src/batch.rs

```rust
//! Batch processing for embeddings

/// Batch processor for OpenAI API calls
pub struct BatchProcessor;

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new() -> Self {
        Self
    }
}
```

### crates/embeddings/src/client.rs

```rust
//! OpenAI embedding client

use anyhow::{Result, anyhow};
use crate::models::{EmbeddingRequest, EmbeddingResponse};
use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::{debug, error};

/// OpenAI embedding client
pub struct EmbeddingClient {
    client: Client,
    api_key: String,
}

impl EmbeddingClient {
    /// Create a new embedding client
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .unwrap_or_else(|_| "dummy-key".to_string()); // Allow dummy key for testing
        
        let client = Client::new();
        
        Ok(Self { client, api_key })
    }
    
    /// Generate embeddings for text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            input: text.to_string(),
            model: "text-embedding-3-large".to_string(),
        };
        
        let response = self.generate_embedding(request).await?;
        Ok(response.embedding)
    }
    
    /// Generate embedding using OpenAI API
    pub async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!("Generating embedding for {} characters", request.input.len());
        
        let payload = json!({
            "input": request.input,
            "model": request.model,
            "encoding_format": "float"
        });
        
        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("OpenAI API error: {}", error_text);
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }
        
        let api_response: serde_json::Value = response.json().await?;
        
        let embedding = api_response
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| anyhow!("Invalid response format from OpenAI API"))?;
        
        let embedding_vec: Result<Vec<f32>, _> = embedding
            .iter()
            .map(|v| v.as_f64().map(|f| f as f32).ok_or_else(|| anyhow!("Invalid embedding value")))
            .collect();
        
        let embedding_vec = embedding_vec?;
        
        debug!("Generated embedding with {} dimensions", embedding_vec.len());
        
        Ok(EmbeddingResponse { embedding: embedding_vec })
    }
}
```

### crates/embeddings/src/lib.rs

```rust
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
```

### crates/embeddings/src/models.rs

```rust
//! Embedding models and types

use serde::{Deserialize, Serialize};

/// Embedding request
#[derive(Debug, Serialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: String,
}

/// Embedding response (simplified for internal use)
#[derive(Debug)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
}

/// OpenAI API embedding response
#[derive(Debug, Deserialize)]
pub struct OpenAIEmbeddingResponse {
    pub data: Vec<EmbeddingData>,
}

/// Embedding data
#[derive(Debug, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
}
```

### crates/mcp/Cargo.toml

```toml
[package]
name = "doc-server-mcp"
version = "0.1.0"
edition = "2021"
description = "MCP (Model Context Protocol) server implementation for the Doc Server"
license = "MIT"

[dependencies]
# Inherit workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
dotenvy = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
rmcp = { workspace = true }

# Additional dependencies for MCP server
async-trait = { workspace = true }
futures = { workspace = true }
chrono = { workspace = true }
async-stream = { workspace = true }

# Local crates
doc-server-database = { path = "../database" }
doc-server-embeddings = { path = "../embeddings" }
doc-server-llm = { path = "../llm" }

[dev-dependencies]
tokio-test = { workspace = true }
mockall = { workspace = true }
```

### crates/mcp/src/handlers.rs

```rust
//! MCP request handlers

use anyhow::{Result, anyhow};
use doc_server_database::DatabasePool;
use crate::tools::{RustQueryTool, Tool};
use serde_json::{Value, json};
use std::collections::HashMap;
use tracing::{debug, error};

/// MCP request handler
pub struct McpHandler {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl McpHandler {
    /// Create a new MCP handler
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();
        
        // Register the rust_query tool
        let rust_query_tool = RustQueryTool::new(db_pool).await?;
        tools.insert("rust_query".to_string(), Box::new(rust_query_tool));
        
        Ok(Self { tools })
    }
    
    /// Handle an MCP request
    pub async fn handle_request(&self, request: Value) -> Result<Value> {
        debug!("Processing MCP request");
        
        // Extract method from request
        let method = request.get("method")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing method in request"))?;
        
        match method {
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(&request).await,
            "initialize" => self.handle_initialize(&request).await,
            _ => Err(anyhow!("Unsupported method: {}", method))
        }
    }
    
    /// Handle tools/list request
    async fn handle_tools_list(&self) -> Result<Value> {
        let tools: Vec<Value> = self.tools.values()
            .map(|tool| tool.definition())
            .collect();
        
        Ok(json!({
            "tools": tools
        }))
    }
    
    /// Handle tools/call request
    async fn handle_tool_call(&self, request: &Value) -> Result<Value> {
        let params = request.get("params")
            .ok_or_else(|| anyhow!("Missing params in tool call"))?;
        
        let tool_name = params.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Missing tool name"))?;
        
        let default_args = json!({});
        let arguments = params.get("arguments")
            .unwrap_or(&default_args);
        
        debug!("Calling tool: {} with arguments: {}", tool_name, arguments);
        
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_name))?;
        
        match tool.execute(arguments.clone()).await {
            Ok(result) => Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": result
                    }
                ]
            })),
            Err(e) => {
                error!("Tool execution failed: {}", e);
                Ok(json!({
                    "content": [
                        {
                            "type": "text", 
                            "text": format!("Error: {}", e)
                        }
                    ],
                    "isError": true
                }))
            }
        }
    }
    
    /// Handle initialize request
    async fn handle_initialize(&self, _request: &Value) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "sse": true
            },
            "serverInfo": {
                "name": "doc-server-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }
}
```

### crates/mcp/src/bin/http_server.rs

```rust
//! HTTP server binary for the Doc Server
//! 
//! This binary provides the main HTTP/SSE endpoint for MCP communication.

use anyhow::Result;
use doc_server_database::{DatabasePool, migrations::Migrations};
use doc_server_mcp::McpServer;
use dotenvy::dotenv;
use std::env;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,doc_server=debug".to_string())
        )
        .init();
    
    info!("Starting Doc Server HTTP server...");
    
    // Get configuration from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");
    
    // Initialize database
    let db_pool = DatabasePool::new(&database_url).await?;
    
    // Run migrations
    if let Err(e) = Migrations::run(db_pool.pool()).await {
        error!("Failed to run migrations: {}", e);
        return Err(e);
    }
    
    // Initialize MCP server
    let mcp_server = McpServer::new(db_pool).await?;
    
    // Start HTTP server
    let addr = format!("0.0.0.0:{}", port);
    info!("Doc Server listening on {}", addr);
    
    mcp_server.serve(&addr).await?;
    
    Ok(())
}
```

### crates/mcp/src/tools.rs

```rust
//! MCP tool definitions

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use doc_server_database::{DatabasePool, queries::DocumentQueries};
use doc_server_embeddings::EmbeddingClient;
use serde_json::{Value, json};
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
    embedding_client: EmbeddingClient,
}

impl RustQueryTool {
    /// Create a new Rust query tool
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let embedding_client = EmbeddingClient::new()?;
        
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
        ).await?;
        
        if results.is_empty() {
            return Ok("No relevant Rust documentation found for your query.".to_string());
        }
        
        // Format results
        let mut response = format!("Found {} relevant Rust documentation results:\n\n", results.len());
        
        for (i, doc) in results.iter().enumerate() {
            let metadata = doc.metadata.as_object()
                .and_then(|m| m.get("crate_name"))
                .and_then(|c| c.as_str())
                .unwrap_or("unknown");
            
            response.push_str(&format!(
                "{}. **{}** (from `{}`)\n{}\n\n",
                i + 1,
                doc.doc_path,
                metadata,
                doc.content.chars().take(300).collect::<String>() + "..."
            ));
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
        let query = arguments.get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| anyhow!("Missing required 'query' parameter"))?;
        
        let limit = arguments.get("limit")
            .and_then(|l| l.as_i64());
        
        // Validate limit
        if let Some(l) = limit {
            if l < 1 || l > 20 {
                return Err(anyhow!("Limit must be between 1 and 20"));
            }
        }
        
        self.semantic_search(query, limit).await
    }
}
```

### crates/mcp/src/transport.rs

```rust
//! MCP transport layer

// TODO: Implement HTTP/SSE transport
```

### crates/mcp/src/lib.rs

```rust
//! MCP (Model Context Protocol) server implementation
//! 
//! This crate provides the MCP server functionality including tool definitions,
//! HTTP/SSE transport, and integration with the database and other services.

pub mod server;
pub mod tools;
pub mod transport;
pub mod handlers;

pub use server::McpServer;

/// Re-export commonly used types
pub use rmcp::*;
```

### crates/mcp/src/server.rs

```rust
//! MCP server implementation

use anyhow::Result;
use doc_server_database::DatabasePool;
use crate::handlers::McpHandler;
use axum::{
    extract::State,
    http::{Method, StatusCode},
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::Stream;
use serde_json::Value;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::time::interval;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error, debug};

/// MCP server state
#[derive(Clone)]
pub struct McpServerState {
    pub db_pool: DatabasePool,
    pub handler: Arc<McpHandler>,
}

/// MCP server
pub struct McpServer {
    state: McpServerState,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new(db_pool: DatabasePool) -> Result<Self> {
        let handler = Arc::new(McpHandler::new(db_pool.clone()).await?);
        let state = McpServerState {
            db_pool,
            handler,
        };
        
        Ok(Self { state })
    }
    
    /// Start serving on the given address
    pub async fn serve(&self, addr: &str) -> Result<()> {
        let app = self.create_router();
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        info!("MCP server listening on {}", addr);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    /// Create the router with all endpoints
    fn create_router(&self) -> Router {
        Router::new()
            // Health check endpoint
            .route("/health", get(health_check))
            // MCP SSE endpoint for real-time communication
            .route("/sse", get(sse_handler))
            // MCP JSON-RPC endpoint for tool calls
            .route("/mcp", post(mcp_handler))
            // Add CORS for Toolman compatibility
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers(Any),
            )
            .with_state(self.state.clone())
    }
}

/// Health check endpoint
async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "doc-server-mcp",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

/// SSE endpoint for real-time communication
async fn sse_handler(
    State(_state): State<McpServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    debug!("New SSE connection established");
    
    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok(Event::default()
            .event("connected")
            .data("{\"status\":\"connected\",\"server\":\"doc-server-mcp\"}")
        );
        
        // Keep-alive heartbeat every 30 seconds
        let mut heartbeat = interval(Duration::from_secs(30));
        
        loop {
            heartbeat.tick().await;
            yield Ok(Event::default()
                .event("heartbeat")
                .data(format!("{{\"timestamp\":{}}}", chrono::Utc::now().timestamp()))
            );
        }
    };
    
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive-text"),
        )
}

/// MCP JSON-RPC handler for tool calls
async fn mcp_handler(
    State(state): State<McpServerState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    debug!("Received MCP request: {}", payload);
    
    match state.handler.handle_request(payload).await {
        Ok(response) => {
            debug!("MCP response: {}", response);
            Ok(Json(response))
        }
        Err(e) => {
            error!("MCP request failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
```

### rustfmt.toml

```toml
# Rust formatting configuration for Doc Server

# Edition and features
edition = "2021"
unstable_features = false

# Code style
max_width = 100
hard_tabs = false
tab_spaces = 4

# Imports
imports_granularity = "Crate"
reorder_imports = true
group_imports = "StdExternalCrate"

# Functions and control flow
fn_args_layout = "Tall"
brace_style = "SameLineWhere"
control_brace_style = "AlwaysSameLine"

# Comments and documentation
wrap_comments = true
format_code_in_doc_comments = true
normalize_comments = true
normalize_doc_attributes = true

# Strings and arrays
format_strings = true
format_macro_matchers = true

# Trailing elements
trailing_comma = "Vertical"
trailing_semicolon = true

# Blank lines
blank_lines_upper_bound = 1
blank_lines_lower_bound = 0

# Misc
newline_style = "Unix"
use_small_heuristics = "Default"
```

### DEV_SETUP.md

```markdown
# Development Setup

This document explains how to quickly start the Doc Server for development.

## Quick Start

### 🚀 Start Development Environment

#### Option 1: Fresh Environment (Empty Database)
```bash
./scripts/dev.sh
```

#### Option 2: With Existing Data (Recommended for Development)
```bash
./scripts/dev.sh --with-data
```

Both scripts will:
1. Start PostgreSQL with pgvector in Docker
2. Wait for the database to be ready
3. Apply database schema and migrations (or load data dump)
4. Start the MCP server on http://localhost:3001

**The `--with-data` option loads a complete database dump containing:**
- 40+ Rust crates with documentation and embeddings
- BirdEye API documentation (600+ endpoints)
- Solana documentation (400+ docs, PDFs, diagrams)
- All vector embeddings ready for semantic search

### 🛑 Stop Development Environment

```bash
./scripts/stop.sh
```

This script will:
1. Stop the PostgreSQL container
2. Kill any running Rust processes
3. Optionally remove the database volume

## Manual Commands

### Database Only

Start just the database (useful if you want to run the server manually):

```bash
docker compose -f docker-compose.dev.yml up -d postgres
```

### Full Production-like Environment

Start everything with Docker Compose (builds the app in Docker too):

```bash
docker compose up -d
```

## Development Workflow

1. **Start the environment**: `./scripts/dev.sh`
2. **Make code changes**: Edit files in `crates/`
3. **Test changes**: The server auto-rebuilds when you restart
4. **Stop when done**: `./scripts/stop.sh`

## Port Configuration

- **MCP Server**: http://localhost:3001
- **Health Check**: http://localhost:3001/health
- **PostgreSQL**: localhost:5433
- **Redis** (optional): localhost:6379

## Database Access

```bash
# Connect to development database
docker compose -f docker-compose.dev.yml exec postgres psql -U docserver -d docs

# Or with psql directly (if installed locally)
psql postgresql://docserver:development_password_change_in_production@localhost:5433/docs
```

## Cursor MCP Configuration

Add to your `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "doc-server": {
      "url": "http://localhost:3001/sse"
    }
  }
}
```

## Troubleshooting

### Database Connection Issues

If you see "role does not exist" errors:
```bash
# Make sure you're using the dev database
unset DATABASE_URL
./scripts/dev.sh
```

### Port Already in Use

If port 3001 is busy:
```bash
# Find what's using the port
lsof -i :3001

# Kill the process
kill -9 <PID>
```

### Docker Issues

```bash
# Reset everything
docker compose -f docker-compose.dev.yml down -v
docker system prune -f
./scripts/dev.sh
```
```

### debug_discovery.py

```python
#!/usr/bin/env python3
"""Debug BirdEye endpoint discovery"""

import requests
from bs4 import BeautifulSoup

class TestDiscovery:
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'gzip, deflate, br',
            'Connection': 'keep-alive',
            'Upgrade-Insecure-Requests': '1',
        })
        self.base_url = "https://docs.birdeye.so"

    def test_discovery(self):
        start_url = f"{self.base_url}/reference/get-defi-price"
        print(f"🔍 Testing discovery with: {start_url}")
        
        try:
            response = self.session.get(start_url, timeout=30)
            response.raise_for_status()
            print(f"✅ Status: {response.status_code}")
            
            soup = BeautifulSoup(response.text, 'html.parser')
            
            endpoint_urls = set()
            total_links = 0
            
            for link in soup.find_all('a', href=True):
                total_links += 1
                href = link['href']
                print(f"  Link {total_links}: {href}")
                
                if href.startswith('/reference/'):
                    full_url = f"{self.base_url}{href}"
                    endpoint_urls.add(full_url)
                    print(f"    ✅ Added: {full_url}")
                
                if total_links >= 10:  # Show first 10 for debugging
                    break
            
            print(f"\n📊 Total links found: {len(soup.find_all('a', href=True))}")
            print(f"📋 Reference URLs found: {len(endpoint_urls)}")
            
            for url in sorted(list(endpoint_urls))[:5]:
                print(f"  - {url}")
                
        except Exception as e:
            print(f"❌ Error: {e}")

if __name__ == "__main__":
    tester = TestDiscovery()
    tester.test_discovery()
```

### docker-compose.dev.yml

```yaml
version: '3.8'

services:
  # PostgreSQL database with pgvector extension
  postgres:
    image: pgvector/pgvector:pg16
    container_name: doc-server-postgres-dev
    environment:
      POSTGRES_DB: docs
      POSTGRES_USER: docserver
      POSTGRES_PASSWORD: development_password_change_in_production
    ports:
      - "5433:5432"
    volumes:
      - postgres_dev_data:/var/lib/postgresql/data
      - ./sql/init:/docker-entrypoint-initdb.d
    networks:
      - doc-server-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U docserver -d docs"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  postgres_dev_data:
    driver: local

networks:
  doc-server-network:
    driver: bridge
```

### README.md

```markdown
# Doc Server

A high-performance documentation search server built in Rust, providing semantic search across multiple documentation types through the Model Context Protocol (MCP).

## 🎯 Overview

Doc Server aggregates and indexes documentation from various sources, enabling AI assistants to perform semantic search across:

- **Rust Crates** - Documentation from docs.rs
- **Jupyter Notebooks** - Interactive notebook documentation
- **Blockchain APIs** - Solana, BirdEye, Meteora, Raydium documentation
- **Infrastructure Tools** - Cilium, Talos Linux, eBPF guides
- **Best Practices** - Curated Rust development guides

## ✨ Key Features

- 🚀 **High Performance** - Built in Rust with async/await
- 🔍 **Semantic Search** - OpenAI embeddings with pgvector
- 🛠️ **MCP Integration** - Native Model Context Protocol support
- 📊 **Type-Specific Tools** - Dedicated query tools for each documentation type
- ⚡ **Batch Processing** - Optimized OpenAI API usage with 70% cost reduction
- 🔄 **SSE Keep-Alive** - Robust connection management for AI clients
- 🐳 **Container Ready** - Docker and Kubernetes deployment support

## 🏗️ Architecture

### Workspace Structure

```
docs/
├── Cargo.toml              # Workspace configuration
├── src/bin/                # Binaries
│   └── http_server.rs      # Main HTTP/SSE server
├── crates/                 # Individual crates
│   ├── database/           # PostgreSQL + pgvector integration
│   ├── mcp/               # MCP protocol implementation
│   ├── embeddings/        # OpenAI embedding generation
│   ├── doc-loader/        # Document parsing and loading
│   └── llm/               # LLM integration for summarization
├── docs/                  # Documentation
└── .taskmaster/           # Project management
```

### Technology Stack

- **Runtime**: Tokio async runtime
- **Database**: PostgreSQL 15+ with pgvector extension
- **Web Framework**: Axum with Tower middleware
- **Embeddings**: OpenAI text-embedding-3-large (3072 dimensions)
- **Protocol**: Model Context Protocol (MCP) over HTTP/SSE
- **Containerization**: Docker with multi-stage builds

## 🚀 Quick Start

### Prerequisites

- Rust 1.83+ 
- PostgreSQL 15+ with pgvector extension
- OpenAI API key

### Local Development

1. **Clone and setup**:
   ```bash
   git clone <repository-url>
   cd docs
   cp .env.example .env
   # Edit .env with your configuration
   ```

2. **Start database**:
   ```bash
   docker-compose up postgres -d
   ```

3. **Run migrations and start server**:
   ```bash
   cargo run --bin http_server
   ```

### Docker Development

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f doc-server
```

### Production Deployment

```bash
# Build optimized image
docker build -t doc-server:latest .

# Deploy with your orchestrator (Kubernetes, Docker Swarm, etc.)
```

## 🔧 Configuration

### Environment Variables

Key configuration options (see `.env.example` for complete list):

```bash
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/docs
OPENAI_API_KEY=sk-your-api-key

# Server
PORT=3000
RUST_LOG=info,doc_server=debug

# Optional optimizations
EMBEDDING_BATCH_SIZE=100
VECTOR_SEARCH_LIMIT=50
```

### Database Setup

```sql
-- Enable extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create database
CREATE DATABASE docs;
```

## 🛠️ MCP Tools

The server exposes type-specific query tools for AI assistants:

### Query Tools
- `rust_query` - Search Rust crate documentation
- `jupyter_query` - Search Jupyter notebook content
- `solana_query` - Search Solana blockchain documentation
- `birdeye_query` - Search BirdEye API documentation
- `meteora_query` - Search Meteora DEX documentation
- `raydium_query` - Search Raydium DEX documentation
- `cilium_query` - Search Cilium networking documentation
- `talos_query` - Search Talos Linux documentation
- `ebpf_query` - Search eBPF programming guides
- `rust_best_practices_query` - Search Rust best practices

### Management Tools (Rust only)
- `add_rust_crate` - Dynamically add new Rust crates
- `remove_rust_crate` - Remove existing crates
- `list_rust_crates` - List available crates
- `check_rust_status` - Check crate indexing status

## 🧪 Development

### Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p doc-server-database

# Build release
cargo build --release --bin http_server
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p doc-server-mcp

# Integration tests
cargo test --test integration
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --all -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## 📚 Documentation

- **[PROJECT_ARCHITECTURE.md](.taskmaster/docs/PROJECT_ARCHITECTURE.md)** - System architecture and design decisions
- **[IMPLEMENTATION_GUIDE.md](.taskmaster/docs/IMPLEMENTATION_GUIDE.md)** - Detailed implementation guide
- **[CONNECTION_RELIABILITY.md](.taskmaster/docs/CONNECTION_RELIABILITY.md)** - SSE connection management
- **[LOCAL_MIGRATION_PLAN.md](.taskmaster/docs/LOCAL_MIGRATION_PLAN.md)** - Database migration procedures

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes following the coding standards
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test --workspace`)
6. Format code (`cargo fmt --all`)
7. Check lints (`cargo clippy --all -- -D warnings`)
8. Commit your changes (`git commit -m 'Add amazing feature'`)
9. Push to the branch (`git push origin feature/amazing-feature`)
10. Open a Pull Request

### Development Guidelines

- Follow Rust community conventions
- Write comprehensive tests for new features
- Update documentation for API changes
- Use meaningful commit messages
- Ensure backward compatibility when possible

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Model Context Protocol](https://github.com/modelcontextprotocol) for the MCP specification
- [pgvector](https://github.com/pgvector/pgvector) for PostgreSQL vector operations
- [OpenAI](https://openai.com) for embedding models
- [Toolman](https://github.com/5dlabs/toolman) for MCP proxy capabilities
```

### scripts/ingestion/ingest_solana.py

```python
#!/usr/bin/env python3
"""
Solana Documentation Ingestion

Ingests all markdown documentation from the Anza-xyz/agave repository
into the Doc Server's harmonized database schema.
"""

import os
import sys
import asyncio
import asyncpg
from typing import List, Dict, Optional
from dataclasses import dataclass
from datetime import datetime
import openai
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector
import re
import json
from pathlib import Path

# Add the project root to Python path for imports
sys.path.append(os.path.join(os.path.dirname(__file__), '../..'))

# Load environment variables
load_dotenv()

@dataclass
class SolanaDocument:
    """Represents a Solana documentation file"""
    file_path: str
    relative_path: str
    title: str
    content: str
    category: str
    metadata: Dict

class SolanaDocProcessor:
    """Processes Solana documentation from the Agave repository"""
    
    def __init__(self, repo_path: str = "solana-agave"):
        self.repo_path = Path(repo_path)
        self.base_path = Path(".")
        
    def categorize_document(self, relative_path: str) -> str:
        """Categorize document based on its path"""
        path_lower = relative_path.lower()
        
        # Main documentation categories
        if "docs/src/consensus" in path_lower:
            return "consensus"
        elif "docs/src/cli" in path_lower:
            return "cli"
        elif "docs/src/validator" in path_lower:
            return "validator"
        elif "docs/src/runtime" in path_lower:
            return "runtime"
        elif "docs/src/proposals" in path_lower:
            return "proposals"
        elif "docs/src/operations" in path_lower:
            return "operations"
        elif "docs/src" in path_lower:
            return "core-docs"
        
        # Module-specific documentation
        elif "readme.md" in path_lower:
            # Extract module name from path
            parts = Path(relative_path).parts
            if len(parts) > 1:
                return f"module-{parts[-2]}"
            return "module-readme"
        
        # Top-level files
        elif relative_path.count("/") == 0:
            return "project-root"
        
        # Default categorization by directory
        else:
            first_dir = Path(relative_path).parts[0]
            return f"module-{first_dir}"
    
    def extract_title_from_content(self, content: str, file_path: str) -> str:
        """Extract title from markdown content"""
        lines = content.strip().split('\n')
        
        # Look for H1 header (# Title)
        for line in lines:
            line = line.strip()
            if line.startswith('# '):
                return line[2:].strip()
        
        # Look for H2 header (## Title) 
        for line in lines:
            line = line.strip()
            if line.startswith('## '):
                return line[3:].strip()
        
        # Use filename if no header found
        filename = Path(file_path).stem
        if filename.lower() == 'readme':
            # Use parent directory name for README files
            parent = Path(file_path).parent.name
            return f"{parent.title()} Module"
        
        return filename.replace('-', ' ').replace('_', ' ').title()
    
    def clean_markdown_content(self, content: str) -> str:
        """Clean and format markdown content for better readability"""
        # Remove excessive whitespace
        content = re.sub(r'\n\s*\n\s*\n', '\n\n', content)
        
        # Fix common markdown issues
        content = re.sub(r'^\s*\n', '', content)  # Remove leading empty lines
        content = content.strip()
        
        return content
    
    def discover_markdown_files(self) -> List[str]:
        """Find all markdown files in the repository"""
        print("🔍 Discovering Solana documentation files...")
        
        md_files = []
        
        # Search for all .md files
        for md_file in self.repo_path.rglob("*.md"):
            # Skip certain directories/files
            relative_path = str(md_file.relative_to(self.repo_path))
            
            # Skip files we don't want
            skip_patterns = [
                'node_modules/',
                '.git/',
                'target/',
                'build/',
                'dist/',
                # Skip files that are likely not documentation
                'CODEOWNERS',
                'NOTICE',
            ]
            
            if any(pattern in relative_path for pattern in skip_patterns):
                continue
                
            md_files.append(str(md_file))
        
        print(f"  📚 Found {len(md_files)} markdown files")
        return md_files
    
    def process_markdown_file(self, file_path: str) -> Optional[SolanaDocument]:
        """Process a single markdown file"""
        try:
            relative_path = str(Path(file_path).relative_to(self.repo_path))
            
            # Read file content
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                raw_content = f.read()
            
            if not raw_content.strip():
                return None
            
            # Clean content
            content = self.clean_markdown_content(raw_content)
            
            # Extract title
            title = self.extract_title_from_content(content, file_path)
            
            # Categorize
            category = self.categorize_document(relative_path)
            
            # Create document
            doc = SolanaDocument(
                file_path=file_path,
                relative_path=relative_path,
                title=title,
                content=content,
                category=category,
                metadata={
                    'source_type': 'markdown',
                    'file_path': relative_path,
                    'category': category,
                    'file_size': len(content),
                    'extracted_at': datetime.utcnow().isoformat(),
                    'repository': 'anza-xyz/agave'
                }
            )
            
            return doc
            
        except Exception as e:
            print(f"  ❌ Error processing {file_path}: {e}")
            return None
    
    def process_all_files(self) -> List[SolanaDocument]:
        """Process all markdown files"""
        print("📝 Processing Solana documentation files...")
        
        md_files = self.discover_markdown_files()
        documents = []
        
        for i, file_path in enumerate(md_files):
            print(f"  Processing {i+1}/{len(md_files)}: {Path(file_path).relative_to(self.repo_path)}")
            
            doc = self.process_markdown_file(file_path)
            if doc:
                documents.append(doc)
        
        print(f"  ✅ Processed {len(documents)} documents successfully")
        
        # Print category summary
        categories = {}
        for doc in documents:
            categories[doc.category] = categories.get(doc.category, 0) + 1
        
        print(f"  📊 Categories found:")
        for category, count in sorted(categories.items()):
            print(f"    - {category}: {count} documents")
        
        return documents

class EmbeddingGenerator:
    """Generates embeddings using OpenAI API"""
    
    def __init__(self, api_key: str):
        self.client = openai.Client(api_key=api_key)
    
    def generate_embedding(self, text: str, doc_title: str = "") -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Truncate text if too long (OpenAI embedding model limit is 8192 tokens)
        MAX_CHARS = 30_000  # ~7500 tokens (conservative estimate: 4 chars/token)
        if len(text) > MAX_CHARS:
            print(f"    ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        try:
            response = self.client.embeddings.create(
                model="text-embedding-3-large",
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            print(f"    ❌ Embedding generation failed for '{doc_title}': {e}")
            raise

class DatabaseManager:
    """Manages database operations"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_documents(self, documents: List[SolanaDocument], embeddings: List[List[float]]):
        """Store documents and embeddings in database"""
        print("💾 Storing in database...")
        
        conn = await asyncpg.connect(self.database_url)
        
        # Register vector type for pgvector
        await register_vector(conn)
        
        try:
            for i, (doc, embedding) in enumerate(zip(documents, embeddings)):
                print(f"  Storing {i+1}/{len(documents)}: {doc.title}")
                
                # Use relative path as doc_path for uniqueness
                doc_path = doc.relative_path
                
                await conn.execute("""
                    INSERT INTO documents (
                        doc_type, source_name, doc_path, content, 
                        metadata, embedding, token_count
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (doc_type, source_name, doc_path) 
                    DO UPDATE SET 
                        content = EXCLUDED.content,
                        metadata = EXCLUDED.metadata,
                        embedding = EXCLUDED.embedding,
                        token_count = EXCLUDED.token_count,
                        updated_at = CURRENT_TIMESTAMP
                """, 
                    'solana',  # doc_type
                    'Solana Agave',  # source_name
                    doc_path,  # doc_path
                    doc.content,  # content
                    json.dumps(doc.metadata),  # metadata (convert dict to JSON string)
                    embedding,  # embedding (pgvector format)
                    len(doc.content) // 4  # estimated token count
                )
            
            print(f"  ✅ Stored {len(documents)} documents in database")
            
        finally:
            await conn.close()

async def main():
    """Main ingestion process"""
    print("🚀 Starting Solana Documentation Ingestion")
    
    # Check environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Check if repository exists
    if not Path("solana-agave").exists():
        print("❌ solana-agave repository not found. Please clone it first:")
        print("   git clone https://github.com/anza-xyz/agave.git solana-agave")
        sys.exit(1)
    
    # Initialize components
    processor = SolanaDocProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Process all documentation
    documents = processor.process_all_files()
    
    if not documents:
        print("❌ No documents found to process")
        return
    
    print(f"📋 Processing {len(documents)} documents...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, doc in enumerate(documents):
        print(f"  Generating embedding {i+1}/{len(documents)} for: {doc.title}")
        embedding = embedding_generator.generate_embedding(doc.content, doc.title)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_documents(documents, embeddings)
    
    print("🎉 Solana documentation ingestion completed successfully!")
    print(f"📊 Summary:")
    print(f"  - Total documents: {len(documents)}")
    print(f"  - Total embeddings: {len(embeddings)}")
    
    # Show category breakdown
    categories = {}
    for doc in documents:
        categories[doc.category] = categories.get(doc.category, 0) + 1
    
    print(f"  - Categories:")
    for category, count in sorted(categories.items()):
        print(f"    * {category}: {count}")

if __name__ == "__main__":
    asyncio.run(main())
```

### scripts/ingestion/requirements.txt

```text
# Python dependencies for documentation ingestion scripts

# Database
asyncpg>=0.29.0

# HTTP requests and async
aiohttp>=3.9.0
requests>=2.31.0

# HTML parsing and data processing
beautifulsoup4>=4.12.0
lxml>=4.9.0

# OpenAI API
openai>=1.7.0

# Utilities
python-dotenv>=1.0.0
```

### scripts/ingestion/ingest_solana_comprehensive.py

```python
#!/usr/bin/env python3
"""
Comprehensive Solana Documentation Ingestion

Ingests ALL documentation from the Anza-xyz/agave repository including:
- Markdown files (.md, .mdx)
- ASCII art diagrams (.bob)  
- Message sequence charts (.msc)
- PDF technical specifications (metadata + basic text if possible)

This enhanced version captures the complete documentation ecosystem.
"""

import os
import sys
import asyncio
import asyncpg
from typing import List, Dict, Optional
from dataclasses import dataclass
from datetime import datetime
import openai
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector
import re
import json
from pathlib import Path

# Add the project root to Python path for imports
sys.path.append(os.path.join(os.path.dirname(__file__), '../..'))

# Load environment variables
load_dotenv()

@dataclass
class SolanaDocument:
    """Represents a Solana documentation file"""
    file_path: str
    relative_path: str
    title: str
    content: str
    file_type: str
    category: str
    metadata: Dict

class ComprehensiveSolanaProcessor:
    """Processes all types of Solana documentation from the Agave repository"""
    
    def __init__(self, repo_path: str = "solana-agave-full"):
        self.repo_path = Path(repo_path)
        self.base_path = Path(".")
        
    def categorize_document(self, relative_path: str, file_type: str) -> str:
        """Enhanced categorization for all file types"""
        path_lower = relative_path.lower()
        
        # Special categories for new file types
        if file_type == "bob":
            return "architecture-diagrams"
        elif file_type == "msc":
            return "sequence-diagrams"
        elif file_type == "pdf":
            if "zk-docs" in path_lower:
                return "zk-cryptography"
            return "technical-specs"
        
        # Existing markdown categorization
        if "docs/src/consensus" in path_lower:
            return "consensus"
        elif "docs/src/cli" in path_lower:
            return "cli"
        elif "docs/src/validator" in path_lower:
            return "validator"
        elif "docs/src/runtime" in path_lower:
            return "runtime"
        elif "docs/src/proposals" in path_lower:
            return "proposals"
        elif "docs/src/operations" in path_lower:
            return "operations"
        elif "docs/src" in path_lower:
            return "core-docs"
        
        # Module-specific documentation
        elif "readme.md" in path_lower:
            parts = Path(relative_path).parts
            if len(parts) > 1:
                return f"module-{parts[-2]}"
            return "module-readme"
        
        # Top-level files
        elif relative_path.count("/") == 0:
            return "project-root"
        
        # Default categorization by directory
        else:
            first_dir = Path(relative_path).parts[0]
            return f"module-{first_dir}"
    
    def extract_title_from_content(self, content: str, file_path: str, file_type: str) -> str:
        """Enhanced title extraction for different file types"""
        
        if file_type in ["md", "mdx"]:
            # Check for frontmatter title first
            if content.startswith('---'):
                frontmatter_end = content.find('---', 3)
                if frontmatter_end != -1:
                    frontmatter = content[3:frontmatter_end]
                    title_match = re.search(r'^title:\s*(.+)$', frontmatter, re.MULTILINE)
                    if title_match:
                        return title_match.group(1).strip('"\'')
            
            # Look for H1 header (# Title)
            lines = content.strip().split('\n')
            for line in lines:
                line = line.strip()
                if line.startswith('# '):
                    return line[2:].strip()
            
            # Look for H2 header (## Title) 
            for line in lines:
                line = line.strip()
                if line.startswith('## '):
                    return line[3:].strip()
        
        elif file_type == "bob":
            # Extract title from BOB diagram comments or filename
            lines = content.strip().split('\n')
            # Look for title in comments
            for line in lines[:5]:  # Check first few lines
                line = line.strip()
                if line.startswith('#') or line.startswith('//'):
                    potential_title = line.lstrip('#/').strip()
                    if len(potential_title) > 3 and len(potential_title) < 100:
                        return potential_title
            
            # Use filename-based title for BOB files
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} Diagram"
        
        elif file_type == "msc":
            # Message Sequence Chart
            lines = content.strip().split('\n')
            for line in lines[:10]:
                if line.strip().startswith('msc') and '{' in line:
                    return "Message Sequence Chart"
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} Sequence"
        
        elif file_type == "pdf":
            # PDF files - use filename
            filename = Path(file_path).stem
            return f"{filename.replace('-', ' ').replace('_', ' ').title()} (PDF)"
        
        # Default fallback
        filename = Path(file_path).stem
        if filename.lower() == 'readme':
            parent = Path(file_path).parent.name
            return f"{parent.title()} Module"
        
        return filename.replace('-', ' ').replace('_', ' ').title()
    
    def clean_content(self, content: str, file_type: str) -> str:
        """Clean content based on file type"""
        if file_type in ["md", "mdx"]:
            # Remove excessive whitespace from markdown
            content = re.sub(r'\n\s*\n\s*\n', '\n\n', content)
            content = re.sub(r'^\s*\n', '', content)
            return content.strip()
        
        elif file_type in ["bob", "msc"]:
            # Preserve ASCII art formatting
            return content.strip()
        
        elif file_type == "pdf":
            # For PDFs, we'll just note it's a PDF - actual text extraction would need additional libraries
            return f"[PDF Document: {Path(content).name}]\n\nThis is a PDF technical specification. Content requires PDF reader to view."
        
        return content.strip()
    
    def discover_documentation_files(self) -> List[str]:
        """Find all documentation files of supported types"""
        print("🔍 Discovering ALL Solana documentation files...")
        
        doc_files = []
        supported_extensions = ['.md', '.mdx', '.bob', '.msc', '.pdf']
        
        # Search for all supported documentation files
        for ext in supported_extensions:
            pattern = f"*{ext}"
            for doc_file in self.repo_path.rglob(pattern):
                relative_path = str(doc_file.relative_to(self.repo_path))
                
                # Skip certain directories/files
                skip_patterns = [
                    'node_modules/',
                    '.git/',
                    'target/',
                    'build/',
                    'dist/',
                    # Skip files that are likely not documentation
                    'CODEOWNERS',
                    'NOTICE',
                ]
                
                if any(pattern in relative_path for pattern in skip_patterns):
                    continue
                    
                doc_files.append(str(doc_file))
        
        # Count by type
        type_counts = {}
        for file_path in doc_files:
            ext = Path(file_path).suffix[1:]  # Remove dot
            type_counts[ext] = type_counts.get(ext, 0) + 1
        
        print(f"  📚 Found {len(doc_files)} documentation files:")
        for file_type, count in sorted(type_counts.items()):
            print(f"    - {file_type}: {count} files")
        
        return doc_files
    
    def process_documentation_file(self, file_path: str) -> Optional[SolanaDocument]:
        """Process a single documentation file of any supported type"""
        try:
            relative_path = str(Path(file_path).relative_to(self.repo_path))
            file_extension = Path(file_path).suffix[1:].lower()  # Remove dot and lowercase
            
            # Read file content based on type
            if file_extension == "pdf":
                # For PDFs, we'll store metadata and reference
                with open(file_path, 'rb') as f:
                    file_size = len(f.read())
                content = file_path  # Store path for reference
            else:
                # Text-based files
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    raw_content = f.read()
                
                if not raw_content.strip():
                    return None
                
                content = self.clean_content(raw_content, file_extension)
            
            # Extract title
            title = self.extract_title_from_content(content, file_path, file_extension)
            
            # Categorize
            category = self.categorize_document(relative_path, file_extension)
            
            # Enhanced metadata
            metadata = {
                'source_type': file_extension,
                'file_path': relative_path,
                'category': category,
                'file_size': len(content) if file_extension != "pdf" else file_size,
                'extracted_at': datetime.utcnow().isoformat(),
                'repository': 'anza-xyz/agave',
                'file_extension': file_extension
            }
            
            # Add specific metadata for PDFs
            if file_extension == "pdf":
                metadata['is_pdf'] = True
                metadata['pdf_path'] = relative_path
                # Create basic content description for PDFs
                content = f"""# {title}

**File Type:** PDF Technical Specification
**Location:** {relative_path}
**Repository:** anza-xyz/agave

This is a PDF document containing technical specifications for Solana's {title.lower()}. The PDF contains detailed mathematical proofs, algorithms, and implementation details.

**To access the full content:** This document requires a PDF reader. The file is located at `{relative_path}` in the Agave repository.

**Category:** Technical specification document
**Format:** Portable Document Format (PDF)
"""
            
            # Create document
            doc = SolanaDocument(
                file_path=file_path,
                relative_path=relative_path,
                title=title,
                content=content,
                file_type=file_extension,
                category=category,
                metadata=metadata
            )
            
            return doc
            
        except Exception as e:
            print(f"  ❌ Error processing {file_path}: {e}")
            return None
    
    def process_all_files(self) -> List[SolanaDocument]:
        """Process all documentation files"""
        print("📝 Processing ALL Solana documentation files...")
        
        doc_files = self.discover_documentation_files()
        documents = []
        
        for i, file_path in enumerate(doc_files):
            print(f"  Processing {i+1}/{len(doc_files)}: {Path(file_path).relative_to(self.repo_path)}")
            
            doc = self.process_documentation_file(file_path)
            if doc:
                documents.append(doc)
        
        print(f"  ✅ Processed {len(documents)} documents successfully")
        
        # Print comprehensive category summary
        categories = {}
        file_types = {}
        for doc in documents:
            categories[doc.category] = categories.get(doc.category, 0) + 1
            file_types[doc.file_type] = file_types.get(doc.file_type, 0) + 1
        
        print(f"  📊 Categories found:")
        for category, count in sorted(categories.items()):
            print(f"    - {category}: {count} documents")
        
        print(f"  📄 File types processed:")
        for file_type, count in sorted(file_types.items()):
            print(f"    - {file_type}: {count} documents")
        
        return documents

class EmbeddingGenerator:
    """Generates embeddings using OpenAI API"""
    
    def __init__(self, api_key: str):
        self.client = openai.Client(api_key=api_key)
    
    def generate_embedding(self, text: str, doc_title: str = "") -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Truncate text if too long (OpenAI embedding model limit is 8192 tokens)
        MAX_CHARS = 30_000  # ~7500 tokens (conservative estimate: 4 chars/token)
        if len(text) > MAX_CHARS:
            print(f"    ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        try:
            response = self.client.embeddings.create(
                model="text-embedding-3-large",
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            print(f"    ❌ Embedding generation failed for '{doc_title}': {e}")
            raise

class DatabaseManager:
    """Manages database operations"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_documents(self, documents: List[SolanaDocument], embeddings: List[List[float]]):
        """Store documents and embeddings in database"""
        print("💾 Storing in database...")
        
        conn = await asyncpg.connect(self.database_url)
        
        # Register vector type for pgvector
        await register_vector(conn)
        
        try:
            for i, (doc, embedding) in enumerate(zip(documents, embeddings)):
                print(f"  Storing {i+1}/{len(documents)}: {doc.title} ({doc.file_type})")
                
                # Use relative path as doc_path for uniqueness
                doc_path = doc.relative_path
                
                await conn.execute("""
                    INSERT INTO documents (
                        doc_type, source_name, doc_path, content, 
                        metadata, embedding, token_count
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (doc_type, source_name, doc_path) 
                    DO UPDATE SET 
                        content = EXCLUDED.content,
                        metadata = EXCLUDED.metadata,
                        embedding = EXCLUDED.embedding,
                        token_count = EXCLUDED.token_count,
                        updated_at = CURRENT_TIMESTAMP
                """, 
                    'solana',  # doc_type
                    'Solana Agave',  # source_name
                    doc_path,  # doc_path
                    doc.content,  # content
                    json.dumps(doc.metadata),  # metadata (convert dict to JSON string)
                    embedding,  # embedding (pgvector format)
                    len(doc.content) // 4  # estimated token count
                )
            
            print(f"  ✅ Stored {len(documents)} documents in database")
            
        finally:
            await conn.close()

async def main():
    """Main ingestion process"""
    print("🚀 Starting COMPREHENSIVE Solana Documentation Ingestion")
    print("📋 Supported formats: Markdown (.md, .mdx), ASCII Diagrams (.bob), Sequence Charts (.msc), PDFs (.pdf)")
    
    # Check environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Check if repository exists
    if not Path("solana-agave-full").exists():
        print("❌ solana-agave-full repository not found. Please clone it first:")
        print("   git clone https://github.com/anza-xyz/agave.git solana-agave-full")
        sys.exit(1)
    
    # Initialize components
    processor = ComprehensiveSolanaProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Process all documentation
    documents = processor.process_all_files()
    
    if not documents:
        print("❌ No documents found to process")
        return
    
    print(f"📋 Processing {len(documents)} comprehensive documents...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, doc in enumerate(documents):
        print(f"  Generating embedding {i+1}/{len(documents)} for: {doc.title} ({doc.file_type})")
        embedding = embedding_generator.generate_embedding(doc.content, doc.title)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_documents(documents, embeddings)
    
    print("🎉 COMPREHENSIVE Solana documentation ingestion completed successfully!")
    print(f"📊 Summary:")
    print(f"  - Total documents: {len(documents)}")
    print(f"  - Total embeddings: {len(embeddings)}")
    
    # Show detailed breakdown
    categories = {}
    file_types = {}
    for doc in documents:
        categories[doc.category] = categories.get(doc.category, 0) + 1
        file_types[doc.file_type] = file_types.get(doc.file_type, 0) + 1
    
    print(f"  - Categories:")
    for category, count in sorted(categories.items()):
        print(f"    * {category}: {count}")
    
    print(f"  - File Types:")
    for file_type, count in sorted(file_types.items()):
        print(f"    * {file_type}: {count}")

if __name__ == "__main__":
    asyncio.run(main())
```

### scripts/ingestion/ingest_birdeye_simple.py

```python
#!/usr/bin/env python3
"""
BirdEye API Documentation Ingestion - Simple Approach

Downloads the complete OpenAPI spec in one request, then processes it locally.
Much more efficient than scraping individual pages.
"""

import json
import os
import sys
import asyncio
import asyncpg
import requests
from typing import List, Dict, Optional
from dataclasses import dataclass
from datetime import datetime
import openai
from dotenv import load_dotenv
from pgvector.asyncpg import register_vector

# Load environment variables
load_dotenv()

@dataclass
class BirdEyeEndpoint:
    """Represents a BirdEye API endpoint"""
    path: str
    method: str
    title: str
    description: str
    content: str
    metadata: Dict

class BirdEyeProcessor:
    """Processes BirdEye API documentation from OpenAPI spec"""
    
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
            'Accept': 'application/json',
            'Accept-Encoding': 'identity',  # Avoid compression issues
        })
    
    def download_openapi_spec(self, output_file: str = "birdeye_openapi.json") -> Dict:
        """Download the complete BirdEye OpenAPI specification"""
        print("🌍 Downloading complete BirdEye OpenAPI specification...")
        
        # Use any endpoint to get the full spec - they all contain the same schema
        url = "https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/get-defi-price?dereference=true&reduce=false"
        
        print(f"  📡 Fetching: {url}")
        response = self.session.get(url, timeout=30)
        response.raise_for_status()
        
        data = response.json()
        
        # Extract the OpenAPI schema
        if 'data' not in data or 'api' not in data['data'] or 'schema' not in data['data']['api']:
            raise Exception("Could not find OpenAPI schema in response")
        
        openapi_spec = data['data']['api']['schema']
        
        # Save to disk
        with open(output_file, 'w') as f:
            json.dump(openapi_spec, f, indent=2)
        
        print(f"  ✅ Saved OpenAPI spec to {output_file}")
        print(f"  📊 Found {len(openapi_spec.get('paths', {}))} API paths")
        
        return openapi_spec
    
    def extract_endpoints_from_spec(self, openapi_spec: Dict) -> List[BirdEyeEndpoint]:
        """Extract individual endpoints from OpenAPI specification"""
        print("🔧 Processing OpenAPI specification...")
        
        endpoints = []
        paths = openapi_spec.get('paths', {})
        
        for path, path_data in paths.items():
            for method, endpoint_data in path_data.items():
                if method.upper() in ['GET', 'POST', 'PUT', 'DELETE', 'PATCH']:
                    
                    title = endpoint_data.get('summary', f'{method.upper()} {path}')
                    description = endpoint_data.get('description', '')
                    
                    # Build comprehensive content for this endpoint
                    content_parts = [
                        f"# {title}",
                        f"**Method:** {method.upper()}",
                        f"**Path:** {path}",
                        ""
                    ]
                    
                    if description:
                        content_parts.extend([f"**Description:** {description}", ""])
                    
                    # Add parameters
                    if 'parameters' in endpoint_data:
                        content_parts.extend(["## Parameters", ""])
                        for param in endpoint_data['parameters']:
                            param_name = param.get('name', 'unknown')
                            param_desc = param.get('description', 'No description')
                            param_required = param.get('required', False)
                            param_location = param.get('in', 'unknown')
                            
                            required_text = " (required)" if param_required else " (optional)"
                            content_parts.append(f"- **{param_name}** ({param_location}){required_text}: {param_desc}")
                        content_parts.append("")
                    
                    # Add response schemas
                    if 'responses' in endpoint_data:
                        content_parts.extend(["## Responses", ""])
                        for status_code, response_data in endpoint_data['responses'].items():
                            response_desc = response_data.get('description', 'No description')
                            content_parts.append(f"- **{status_code}**: {response_desc}")
                        content_parts.append("")
                    
                    # Add complete endpoint schema as JSON (but limit size)
                    content_parts.extend(["## Complete Schema", "```json"])
                    endpoint_json = json.dumps(endpoint_data, indent=2)
                    if len(endpoint_json) > 20_000:  # Limit to ~5K tokens
                        endpoint_json = endpoint_json[:20_000] + "... [TRUNCATED]"
                    content_parts.append(endpoint_json)
                    content_parts.extend(["```", ""])
                    
                    full_content = "\n".join(content_parts)
                    
                    endpoint = BirdEyeEndpoint(
                        path=path,
                        method=method.upper(),
                        title=title,
                        description=description,
                        content=full_content,
                        metadata={
                            'source': 'birdeye_openapi',
                            'extracted_at': datetime.utcnow().isoformat(),
                            'endpoint_data': endpoint_data
                        }
                    )
                    
                    endpoints.append(endpoint)
        
        print(f"  ✅ Extracted {len(endpoints)} endpoints")
        return endpoints

class EmbeddingGenerator:
    """Generates embeddings using OpenAI API"""
    
    def __init__(self, api_key: str):
        self.client = openai.Client(api_key=api_key)
    
    def generate_embedding(self, text: str) -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Ensure text fits in embedding model limits
        MAX_CHARS = 30_000  # ~7500 tokens
        if len(text) > MAX_CHARS:
            print(f"  ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        try:
            response = self.client.embeddings.create(
                model="text-embedding-3-large",
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            print(f"  ❌ Embedding generation failed: {e}")
            raise

class DatabaseManager:
    """Manages database operations"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_endpoints(self, endpoints: List[BirdEyeEndpoint], embeddings: List[List[float]]):
        """Store endpoints and embeddings in database"""
        print("💾 Storing in database...")
        
        conn = await asyncpg.connect(self.database_url)
        
        # Register vector type for pgvector
        await register_vector(conn)
        
        try:
            for i, (endpoint, embedding) in enumerate(zip(endpoints, embeddings)):
                print(f"  Storing {i+1}/{len(endpoints)}: {endpoint.title}")
                
                # Create doc_path as method + path
                doc_path = f"{endpoint.method} {endpoint.path}"
                
                await conn.execute("""
                    INSERT INTO documents (
                        doc_type, source_name, doc_path, content, 
                        metadata, embedding, token_count
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (doc_type, source_name, doc_path) 
                    DO UPDATE SET 
                        content = EXCLUDED.content,
                        metadata = EXCLUDED.metadata,
                        embedding = EXCLUDED.embedding,
                        token_count = EXCLUDED.token_count,
                        updated_at = CURRENT_TIMESTAMP
                """, 
                    'birdeye',  # doc_type
                    'BirdEye API',  # source_name
                    doc_path,  # doc_path
                    endpoint.content,  # content
                    json.dumps(endpoint.metadata),  # metadata
                    embedding,  # embedding (pgvector format)
                    len(endpoint.content) // 4  # estimated token count
                )
            
            print(f"  ✅ Stored {len(endpoints)} endpoints in database")
            
        finally:
            await conn.close()

async def main():
    """Main ingestion process"""
    print("🚀 Starting BirdEye API Documentation Ingestion (Simple Approach)")
    
    # Check environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Initialize components
    processor = BirdEyeProcessor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Download OpenAPI spec (one request!)
    openapi_spec = processor.download_openapi_spec()
    
    # Extract individual endpoints
    endpoints = processor.extract_endpoints_from_spec(openapi_spec)
    
    print(f"📋 Processing {len(endpoints)} endpoints...")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, endpoint in enumerate(endpoints):
        print(f"  Generating embedding {i+1}/{len(endpoints)} for: {endpoint.title}")
        embedding = embedding_generator.generate_embedding(endpoint.content)
        embeddings.append(embedding)
    
    # Store in database
    await db_manager.store_endpoints(endpoints, embeddings)
    
    print("🎉 BirdEye ingestion completed successfully!")

if __name__ == "__main__":
    asyncio.run(main())
```

### scripts/ingestion/ingest_birdeye.py

```python
#!/usr/bin/env python3
"""
BirdEye API Documentation Ingestion Script

Extracts BirdEye API documentation from their embedded JSON and stores it
in the Doc Server harmonized database schema with embeddings.

This script automatically discovers ALL BirdEye API endpoints by scraping
their documentation navigation, then extracts the embedded JSON from each page.

Based on the WIP extract_birdeye_json.py approach, enhanced for comprehensive coverage.
"""

import urllib.parse
import html
import json
import requests
import time
import sys
import os
import asyncio
import asyncpg
from typing import List, Dict, Optional, Set
from dataclasses import dataclass
from datetime import datetime
import uuid
import re
from bs4 import BeautifulSoup

# Add the project root to Python path for imports
sys.path.append(os.path.join(os.path.dirname(__file__), '../..'))

@dataclass
class BirdEyeEndpoint:
    """Represents a BirdEye API endpoint"""
    url: str
    title: str
    method: str
    path: str
    content: str
    metadata: Dict

class BirdEyeExtractor:
    """Extracts BirdEye API documentation from their docs pages"""
    
    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            'Accept': 'application/json',
            'Accept-Language': 'en-US,en;q=0.5',
            'Accept-Encoding': 'identity',  # No compression to avoid Brotli issues
            'Connection': 'keep-alive',
        })
        self.base_url = "https://docs.birdeye.so"
    
    def discover_all_endpoints(self) -> List[str]:
        """Get all BirdEye API endpoints (using curated list since site is React-based)"""
        print("🔍 Getting BirdEye API endpoints...")
        
        # BirdEye uses React/JS rendering, so use curated comprehensive endpoint list
        print("  📋 Using comprehensive endpoint list (React site - JS rendered navigation)")
        return [
            # Core price endpoints
            "https://docs.birdeye.so/reference/get-defi-price",
            "https://docs.birdeye.so/reference/get-defi-multi_price", 
            "https://docs.birdeye.so/reference/post-defi-multi_price",
            "https://docs.birdeye.so/reference/get-defi-historical_price_unix",
            "https://docs.birdeye.so/reference/get-defi-history_price",
            "https://docs.birdeye.so/reference/get-defi-price_volume-single",
            "https://docs.birdeye.so/reference/post-defi-price_volume-multi",
            
            # Trading data endpoints  
            "https://docs.birdeye.so/reference/get-defi-txs-token",
            "https://docs.birdeye.so/reference/get-defi-txs-pair",
            "https://docs.birdeye.so/reference/get-defi-txs-token-seek_by_time",
            "https://docs.birdeye.so/reference/get-defi-txs-pair-seek_by_time",
            "https://docs.birdeye.so/reference/get-defi-v3-txs",
            "https://docs.birdeye.so/reference/get-defi-v3-token-txs", 
            "https://docs.birdeye.so/reference/get-defi-v3-txs-recent",
            
            # OHLCV endpoints
            "https://docs.birdeye.so/reference/get-defi-ohlcv",
            "https://docs.birdeye.so/reference/get-defi-ohlcv-pair",
            "https://docs.birdeye.so/reference/get-defi-ohlcv-base_quote",
            "https://docs.birdeye.so/reference/get-defi-v3-ohlcv",
            "https://docs.birdeye.so/reference/get-defi-v3-ohlcv-pair",
            
            # Token endpoints
            "https://docs.birdeye.so/reference/get-defi-token_overview",
            "https://docs.birdeye.so/reference/get-defi-token_security",
            "https://docs.birdeye.so/reference/get-defi-token_creation_info", 
            "https://docs.birdeye.so/reference/get-defi-tokenlist",
            "https://docs.birdeye.so/reference/get-defi-token_trending",
            "https://docs.birdeye.so/reference/get-defi-v3-token-list",
            "https://docs.birdeye.so/reference/get-defi-v3-token-list-scroll",
            "https://docs.birdeye.so/reference/get-defi-v3-token-meta-data-single",
            "https://docs.birdeye.so/reference/get-defi-v3-token-meta-data-multiple",
            "https://docs.birdeye.so/reference/get-defi-v3-token-market-data",
            "https://docs.birdeye.so/reference/get-defi-v3-token-market-data-multiple",
            "https://docs.birdeye.so/reference/get-defi-v3-token-trade-data-single",
            "https://docs.birdeye.so/reference/get-defi-v3-token-trade-data-multiple",
            
            # Wallet endpoints
            "https://docs.birdeye.so/reference/get-trader-gainers-losers",
            "https://docs.birdeye.so/reference/get-trader-txs-seek_by_time",
            "https://docs.birdeye.so/reference/get-v1-wallet-balance_change",
            "https://docs.birdeye.so/reference/get-v1-wallet-portfolio",
            "https://docs.birdeye.so/reference/get-v1-wallet-token_balance",
            "https://docs.birdeye.so/reference/get-v1-wallet-tx_list",
            "https://docs.birdeye.so/reference/get-v1-wallet-net_worth",
            
            # Utility endpoints
            "https://docs.birdeye.so/reference/get-defi-v3-search", 
            "https://docs.birdeye.so/reference/get-defi-networks",
            "https://docs.birdeye.so/reference/get-v1-wallet-list_supported_chain",
        ]
    
    def extract_endpoint_data(self, url: str) -> Optional[BirdEyeEndpoint]:
        """Extract API documentation from a BirdEye docs URL using dereference API"""
        try:
            # Convert regular docs URL to dereference API URL
            # From: https://docs.birdeye.so/reference/get-defi-price
            # To: https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/get-defi-price?dereference=true&reduce=false
            
            if '/reference/' not in url:
                print(f"  ❌ Invalid URL format: {url}")
                return None
                
            # Extract the endpoint slug
            slug = url.split('/reference/')[-1]
            api_url = f"https://docs.birdeye.so/birdeyedotso/api-next/v2/branches/1.0/reference/{slug}?dereference=true&reduce=false"
            
            print(f"  📡 Fetching: {api_url}")
            response = self.session.get(api_url, timeout=30)
            response.raise_for_status()
            
            # Parse JSON response directly
            data = response.json()
            
            # Extract clean API documentation from dereference response
            if 'data' not in data:
                print(f"  ❌ No data section in response")
                return None
                
            data_section = data['data']
            title = data_section.get('title', 'Unknown Endpoint')
            
            # Note: Removed overly aggressive rate limit detection
            # Those keywords appear in OpenAPI error response descriptions, not actual rate limiting
            
            # Extract method and path from API section
            method = "GET"  # Default
            path = "/unknown"
            api_spec = {}
            
            if 'api' in data_section:
                api_info = data_section['api']
                method = api_info.get('method', 'GET').upper()
                path = api_info.get('path', '/unknown')
                
                # Get the full OpenAPI schema if available
                if 'schema' in api_info:
                    api_spec = api_info['schema']
            
            # Build comprehensive content
            content_parts = []
            
            # Add title and basic info
            content_parts.append(f"# {title}")
            content_parts.append(f"**Method:** {method}")
            content_parts.append(f"**Path:** {path}")
            content_parts.append("")
            
            # Add content body/description from the dereference API
            content_body = ""
            if 'content' in data_section and 'body' in data_section['content']:
                content_body = data_section['content']['body']
                content_parts.append(f"**Description:**\n{content_body}")
                content_parts.append("")
            
            # Add OpenAPI specification if available
            if api_spec:
                content_parts.append("## OpenAPI Specification")
                content_parts.append("```json")
                content_parts.append(json.dumps(api_spec, indent=2))
                content_parts.append("```")
                content_parts.append("")
            
            # Add metadata information
            if 'metadata' in data_section:
                metadata_info = data_section['metadata']
                if metadata_info:
                    content_parts.append("## Metadata")
                    for key, value in metadata_info.items():
                        if value:
                            content_parts.append(f"**{key.title()}:** {value}")
                    content_parts.append("")
            
            full_content = "\n".join(content_parts)
            
            endpoint = BirdEyeEndpoint(
                url=url,
                title=title,
                method=method,
                path=path,
                content=full_content,
                metadata={
                    'source_url': url,
                    'api_url': api_url,
                    'api_method': method,
                    'api_path': path,
                    'title': title,
                    'openapi_spec': api_spec,
                    'content_body': content_body,
                    'extracted_at': datetime.utcnow().isoformat()
                }
            )
            
            print(f"  ✅ Extracted: {title} ({method} {path})")
            return endpoint
            
        except Exception as e:
            print(f"  ❌ Error processing {url}: {e}")
            return None

class EmbeddingGenerator:
    """Generates OpenAI embeddings for text content"""
    
    def __init__(self, api_key: str):
        self.api_key = api_key
        self.base_url = "https://api.openai.com/v1"
    
    async def generate_embedding(self, text: str) -> List[float]:
        """Generate embedding for text using OpenAI API"""
        # Truncate text if too long (OpenAI embedding model limit is 8192 tokens)
        MAX_CHARS = 30_000  # ~7500 tokens (conservative estimate: 4 chars/token)
        if len(text) > MAX_CHARS:
            print(f"  ⚠️  Truncating content from {len(text):,} to {MAX_CHARS:,} characters")
            text = text[:MAX_CHARS] + "... [TRUNCATED]"
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        payload = {
            "model": "text-embedding-3-large",
            "input": text,
            "encoding_format": "float"
        }
        
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{self.base_url}/embeddings",
                headers=headers,
                json=payload
            ) as response:
                if response.status != 200:
                    error_text = await response.text()
                    print(f"❌ OpenAI API Error {response.status}: {error_text}")
                    raise Exception(f"OpenAI API error: {response.status} - {error_text}")
                
                result = await response.json()
                return result['data'][0]['embedding']

class DatabaseManager:
    """Manages database operations for BirdEye documentation"""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
    
    async def store_endpoints(self, endpoints: List[BirdEyeEndpoint], embeddings: List[List[float]]):
        """Store BirdEye endpoints in the database"""
        conn = await asyncpg.connect(self.database_url)
        
        try:
            # First, create or update the document source
            await self.ensure_document_source(conn)
            
            # Store individual documents
            for endpoint, embedding in zip(endpoints, embeddings):
                await self.store_document(conn, endpoint, embedding)
            
            # Update source statistics
            await self.update_source_stats(conn)
            
        finally:
            await conn.close()
    
    async def ensure_document_source(self, conn):
        """Ensure BirdEye document source exists"""
        await conn.execute('''
            INSERT INTO document_sources (
                doc_type, source_name, version, config, enabled
            ) VALUES (
                'birdeye', 'birdeye-api', 'latest',
                $1, true
            ) ON CONFLICT (doc_type, source_name) DO UPDATE SET
                config = $1,
                updated_at = CURRENT_TIMESTAMP
        ''', json.dumps({
            'base_url': 'https://docs.birdeye.so',
            'extraction_method': 'data-initial-props scraping',
            'last_ingestion': datetime.utcnow().isoformat()
        }))
    
    async def store_document(self, conn, endpoint: BirdEyeEndpoint, embedding: List[float]):
        """Store a single BirdEye endpoint document"""
        doc_path = f"{endpoint.method.lower()}{endpoint.path.replace('/', '_')}"
        
        await conn.execute('''
            INSERT INTO documents (
                doc_type, source_name, doc_path, content, metadata, embedding, token_count
            ) VALUES (
                'birdeye', 'birdeye-api', $1, $2, $3, $4, $5
            ) ON CONFLICT (doc_type, source_name, doc_path) DO UPDATE SET
                content = $2,
                metadata = $3,
                embedding = $4,
                token_count = $5,
                updated_at = CURRENT_TIMESTAMP
        ''', doc_path, endpoint.content, json.dumps(endpoint.metadata), 
             embedding, len(endpoint.content.split()))
    
    async def update_source_stats(self, conn):
        """Update document source statistics"""
        await conn.execute('''
            UPDATE document_sources 
            SET 
                total_docs = (
                    SELECT COUNT(*) 
                    FROM documents 
                    WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
                ),
                total_tokens = (
                    SELECT COALESCE(SUM(token_count), 0) 
                    FROM documents 
                    WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
                )
            WHERE doc_type = 'birdeye' AND source_name = 'birdeye-api'
        ''')

async def main():
    """Main ingestion workflow"""
    print("🚀 Starting BirdEye API Documentation Ingestion")
    
    # Check for required environment variables
    database_url = os.getenv('DATABASE_URL')
    openai_api_key = os.getenv('OPENAI_API_KEY')
    
    if not database_url:
        print("❌ DATABASE_URL environment variable required")
        sys.exit(1)
    
    if not openai_api_key:
        print("❌ OPENAI_API_KEY environment variable required")
        sys.exit(1)
    
    # Initialize components
    extractor = BirdEyeExtractor()
    embedding_generator = EmbeddingGenerator(openai_api_key)
    db_manager = DatabaseManager(database_url)
    
    # Discover all BirdEye API endpoints automatically
    all_endpoints = extractor.discover_all_endpoints()
    
    # For testing: limit to first 3 endpoints (set to None for full run)
    test_mode = True  # Change to False for full ingestion
    if test_mode:
        endpoints_to_extract = all_endpoints[:3]
        print(f"🧪 TEST MODE: Processing only {len(endpoints_to_extract)} endpoints")
        for url in endpoints_to_extract:
            print(f"  - {url}")
    else:
        endpoints_to_extract = all_endpoints
    
    # Extract endpoints
    print(f"📋 Extracting {len(endpoints_to_extract)} BirdEye endpoints...")
    endpoints = []
    
    for i, url in enumerate(endpoints_to_extract):
        print(f"Processing {i+1}/{len(endpoints_to_extract)}: {url}")
        
        endpoint = extractor.extract_endpoint_data(url)
        if endpoint:
            endpoints.append(endpoint)
        
        # Rate limiting - wait between requests to be respectful
        if i < len(endpoints_to_extract) - 1:
            delay = 15 if len(endpoints_to_extract) > 20 else 10
            print(f"  💤 Waiting {delay} seconds...")
            time.sleep(delay)
    
    if not endpoints:
        print("❌ No endpoints successfully extracted")
        sys.exit(1)
    
    print(f"✅ Extracted {len(endpoints)} endpoints")
    
    # Generate embeddings
    print("🔮 Generating embeddings...")
    embeddings = []
    for i, endpoint in enumerate(endpoints):
        print(f"  Generating embedding {i+1}/{len(endpoints)} for: {endpoint.title}")
        embedding = await embedding_generator.generate_embedding(endpoint.content)
        embeddings.append(embedding)
        
        # Small delay to respect rate limits
        await asyncio.sleep(1)
    
    print(f"✅ Generated {len(embeddings)} embeddings")
    
    # Store in database
    print("💾 Storing in database...")
    await db_manager.store_endpoints(endpoints, embeddings)
    
    print("🎉 BirdEye ingestion completed successfully!")
    print(f"📊 Ingested {len(endpoints)} BirdEye API endpoints")

if __name__ == "__main__":
    # Import aiohttp here to avoid issues if not installed
    try:
        import aiohttp
    except ImportError:
        print("❌ aiohttp required. Install with: pip install aiohttp")
        sys.exit(1)
    
    try:
        import asyncpg
    except ImportError:
        print("❌ asyncpg required. Install with: pip install asyncpg")
        sys.exit(1)
    
    asyncio.run(main())
```

### scripts/dev.sh

```bash
#!/bin/bash

# Development environment startup script for Doc Server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting Doc Server Development Environment${NC}"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for --with-data flag
LOAD_DATA=false
if [[ "$1" == "--with-data" ]]; then
    LOAD_DATA=true
    echo -e "${BLUE}🗂️  Will load database dump with existing documentation${NC}"
fi

# Check for required tools
if ! command_exists docker; then
    echo -e "${RED}❌ Docker is required but not installed. Please install Docker.${NC}"
    exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
    echo -e "${RED}❌ Docker Compose is required but not installed. Please install Docker Compose.${NC}"
    exit 1
fi

# Start the database
echo -e "${YELLOW}📦 Starting PostgreSQL database...${NC}"
docker compose -f docker-compose.dev.yml up -d postgres

# Wait for database to be ready
echo -e "${YELLOW}⏳ Waiting for database to be ready...${NC}"
until docker compose -f docker-compose.dev.yml exec postgres pg_isready -U docserver -d docs; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done

echo -e "${GREEN}✅ Database is ready!${NC}"

# Run migrations
echo -e "${YELLOW}🔄 Running database migrations...${NC}"
if [ -f .env ]; then
    export $(cat .env | xargs)
fi

# Make sure we have the database URL for local development
export DATABASE_URL="postgresql://docserver:development_password_change_in_production@localhost:5433/docs"

# Run Rust migrations
cargo run --bin migrations 2>/dev/null || echo -e "${YELLOW}⚠️  No migrations binary found, skipping Rust migrations${NC}"

# Run SQL schema if it exists (only if not loading data dump)
if [ "$LOAD_DATA" = false ] && [ -f sql/schema.sql ]; then
    echo -e "${YELLOW}📋 Applying SQL schema...${NC}"
    docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs < sql/schema.sql
fi

# Load database dump if requested
if [ "$LOAD_DATA" = true ]; then
    if [ -f sql/data/docs_database_dump.sql.gz ]; then
        echo -e "${YELLOW}📊 Loading database dump with existing documentation...${NC}"
        echo -e "${BLUE}This includes 40+ Rust crates, BirdEye docs, and Solana docs with embeddings${NC}"
        gunzip -c sql/data/docs_database_dump.sql.gz | docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs
        echo -e "${GREEN}✅ Database dump loaded successfully!${NC}"
    else
        echo -e "${RED}❌ Database dump not found at sql/data/docs_database_dump.sql.gz${NC}"
        echo -e "${YELLOW}💡 Continuing with empty database...${NC}"
    fi
fi

echo -e "${GREEN}✅ Database setup complete!${NC}"

# Start the MCP server
echo -e "${YELLOW}🖥️  Starting MCP server...${NC}"
echo -e "${BLUE}Server will be available at: http://localhost:3001${NC}"
echo -e "${BLUE}Health check: http://localhost:3001/health${NC}"
echo -e "${BLUE}Press Ctrl+C to stop${NC}"

# Start the server (this will run in foreground)
unset DATABASE_URL
cargo run -p doc-server-mcp --bin http_server
```

### scripts/run_birdeye_ingestion.sh

```bash
#!/bin/bash

# BirdEye Documentation Ingestion Runner
# This script sets up the environment and runs the BirdEye ingestion

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🐦 Starting BirdEye Documentation Ingestion${NC}"

# Check if we're in the right directory
if [ ! -f "scripts/ingestion/ingest_birdeye.py" ]; then
    echo -e "${RED}❌ Please run this script from the project root directory${NC}"
    exit 1
fi

# Load environment variables from .env if it exists
if [ -f ".env" ]; then
    echo -e "${YELLOW}📋 Loading environment variables from .env${NC}"
    export $(cat .env | grep -v '#' | xargs)
else
    echo -e "${YELLOW}⚠️  No .env file found. Ensure DATABASE_URL and OPENAI_API_KEY are set${NC}"
fi

# Check required environment variables
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}❌ DATABASE_URL environment variable is required${NC}"
    echo "Set it in .env or export DATABASE_URL=postgresql://user:pass@localhost:5432/docs"
    exit 1
fi

if [ -z "$OPENAI_API_KEY" ]; then
    echo -e "${RED}❌ OPENAI_API_KEY environment variable is required${NC}"
    echo "Set it in .env or export OPENAI_API_KEY=sk-your-key-here"
    exit 1
fi

# Check if Python dependencies are installed
echo -e "${YELLOW}🔍 Checking Python dependencies...${NC}"
if ! python3 -c "import asyncpg, aiohttp, requests" 2>/dev/null; then
    echo -e "${YELLOW}📦 Installing Python dependencies...${NC}"
    pip3 install -r scripts/ingestion/requirements.txt
    echo -e "${GREEN}✅ Dependencies installed${NC}"
else
    echo -e "${GREEN}✅ Dependencies already installed${NC}"
fi

# Test database connection
echo -e "${YELLOW}🔍 Testing database connection...${NC}"
if ! python3 -c "
import asyncio
import asyncpg
import os
async def test():
    conn = await asyncpg.connect(os.getenv('DATABASE_URL'))
    await conn.execute('SELECT 1')
    await conn.close()
    print('Database connection successful')
asyncio.run(test())
" 2>/dev/null; then
    echo -e "${RED}❌ Cannot connect to database. Please check DATABASE_URL${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Database connection successful${NC}"
fi

# Run the ingestion
echo -e "${GREEN}🚀 Starting BirdEye ingestion...${NC}"
python3 scripts/ingestion/ingest_birdeye.py

echo -e "${GREEN}🎉 BirdEye ingestion completed!${NC}"

# Show results
echo -e "${YELLOW}📊 Checking ingestion results...${NC}"
python3 -c "
import asyncio
import asyncpg
import os
async def check_results():
    conn = await asyncpg.connect(os.getenv('DATABASE_URL'))
    
    # Check document source
    source = await conn.fetchrow('''
        SELECT source_name, total_docs, total_tokens, last_updated 
        FROM document_sources 
        WHERE doc_type = 'birdeye'
    ''')
    
    if source:
        print(f'📋 Document Source: {source[\"source_name\"]}')
        print(f'📄 Total Documents: {source[\"total_docs\"]}')
        print(f'🔤 Total Tokens: {source[\"total_tokens\"]}')
        print(f'🕒 Last Updated: {source[\"last_updated\"]}')
    
    # Check sample documents
    docs = await conn.fetch('''
        SELECT doc_path, token_count 
        FROM documents 
        WHERE doc_type = 'birdeye' 
        ORDER BY created_at 
        LIMIT 5
    ''')
    
    print('\\n📄 Sample Documents:')
    for doc in docs:
        print(f'  - {doc[\"doc_path\"]} ({doc[\"token_count\"]} tokens)')
    
    await conn.close()

asyncio.run(check_results())
"

echo -e "${GREEN}✅ BirdEye documentation ingestion completed successfully!${NC}"
```

### scripts/backup_database.sh

```bash
#!/bin/bash

# Backup script for rust_docs_vectors database
# Creates timestamped backups before migration

set -e

# Configuration
DB_NAME="rust_docs_vectors"
DB_USER="jonathonfritz"
DB_HOST="localhost"
BACKUP_DIR="$HOME/backups/rust_docs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="$BACKUP_DIR/$TIMESTAMP"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Doc Server Database Backup ===${NC}"
echo "Database: $DB_NAME"
echo "Backup location: $BACKUP_PATH"
echo

# Create backup directory
mkdir -p "$BACKUP_PATH"

# Create full database backup
echo -e "${YELLOW}Creating full database backup...${NC}"
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -Fc > "$BACKUP_PATH/${DB_NAME}_full.dump"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.dump${NC}"

# Also create SQL format for easy inspection
echo -e "${YELLOW}Creating SQL format backup...${NC}"
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/${DB_NAME}_full.sql"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.sql${NC}"

# Export data as CSV for extra safety
echo -e "${YELLOW}Exporting data as CSV...${NC}"
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" << EOF
\COPY crates TO '$BACKUP_PATH/crates_backup.csv' WITH CSV HEADER
\COPY doc_embeddings TO '$BACKUP_PATH/doc_embeddings_backup.csv' WITH CSV HEADER
EOF
echo -e "${GREEN}✓ Created: crates_backup.csv${NC}"
echo -e "${GREEN}✓ Created: doc_embeddings_backup.csv${NC}"

# Get database statistics
echo -e "${YELLOW}Capturing database statistics...${NC}"
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/db_stats.txt" << 'EOF'
\echo 'DATABASE STATISTICS'
\echo '=================='
\echo ''
\echo 'Table Sizes:'
\dt+
\echo ''
\echo 'Row Counts:'
SELECT 'crates' as table_name, COUNT(*) as row_count FROM crates
UNION ALL
SELECT 'doc_embeddings', COUNT(*) FROM doc_embeddings;
\echo ''
\echo 'Crate Summary:'
SELECT COUNT(DISTINCT name) as total_crates, SUM(total_docs) as total_docs FROM crates;
\echo ''
\echo 'Top 10 Crates by Document Count:'
SELECT c.name, COUNT(de.id) as doc_count 
FROM crates c 
LEFT JOIN doc_embeddings de ON c.id = de.crate_id 
GROUP BY c.name 
ORDER BY doc_count DESC 
LIMIT 10;
EOF
echo -e "${GREEN}✓ Created: db_stats.txt${NC}"

# Create restore script
cat > "$BACKUP_PATH/restore.sh" << 'EOF'
#!/bin/bash
# Restore script for this backup

if [ "$1" = "" ]; then
    echo "Usage: $0 <target_database_name>"
    echo "Example: $0 rust_docs_vectors_restored"
    exit 1
fi

TARGET_DB=$1
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "This will create a new database: $TARGET_DB"
echo "Press Enter to continue or Ctrl+C to cancel..."
read

createdb "$TARGET_DB"
pg_restore -h localhost -U jonathonfritz -d "$TARGET_DB" "$SCRIPT_DIR/rust_docs_vectors_full.dump"

echo "Restore complete. Verify with:"
echo "psql -d $TARGET_DB -c 'SELECT COUNT(*) FROM doc_embeddings;'"
EOF
chmod +x "$BACKUP_PATH/restore.sh"
echo -e "${GREEN}✓ Created: restore.sh${NC}"

# Calculate backup size
BACKUP_SIZE=$(du -sh "$BACKUP_PATH" | cut -f1)

echo
echo -e "${GREEN}=== Backup Complete ===${NC}"
echo "Location: $BACKUP_PATH"
echo "Size: $BACKUP_SIZE"
echo "Files created:"
ls -la "$BACKUP_PATH"
echo
echo -e "${YELLOW}To restore this backup later:${NC}"
echo "cd $BACKUP_PATH"
echo "./restore.sh <new_database_name>"
```

### scripts/setup_database.sh

```bash
#!/bin/bash

# Setup script for Doc Server database
# This script creates the new harmonized database and optionally migrates data

set -e  # Exit on any error

# Configuration
DB_NAME="docs"
OLD_DB_NAME="rust_docs_vectors" 
DB_USER="${DB_USER:-$(whoami)}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Setting up Doc Server database...${NC}"

# Function to run SQL and show result
run_sql() {
    local description="$1"
    local sql_file="$2"
    echo -e "${YELLOW}📝 ${description}...${NC}"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$sql_file"
    echo -e "${GREEN}✅ ${description} completed${NC}"
}

# Function to run SQL command directly
run_sql_cmd() {
    local description="$1"
    local sql_cmd="$2"
    echo -e "${YELLOW}📝 ${description}...${NC}"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "$sql_cmd"
    echo -e "${GREEN}✅ ${description} completed${NC}"
}

# Check if PostgreSQL is running
echo -e "${YELLOW}🔍 Checking PostgreSQL connection...${NC}"
if ! psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres -c "SELECT 1;" > /dev/null 2>&1; then
    echo -e "${RED}❌ Cannot connect to PostgreSQL. Please ensure it's running.${NC}"
    echo "Try: docker-compose up postgres -d"
    exit 1
fi

# Check if old database exists for migration
OLD_DB_EXISTS=false
if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -lqt | cut -d \| -f 1 | grep -qw "$OLD_DB_NAME"; then
    OLD_DB_EXISTS=true
    echo -e "${GREEN}✅ Found existing $OLD_DB_NAME database for migration${NC}"
else
    echo -e "${YELLOW}ℹ️  No existing $OLD_DB_NAME database found, skipping migration${NC}"
fi

# Create new database if it doesn't exist
echo -e "${YELLOW}🗄️  Creating database '$DB_NAME'...${NC}"
if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -lqt | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
    echo -e "${YELLOW}⚠️  Database '$DB_NAME' already exists. Continue? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "Exiting..."
        exit 0
    fi
else
    createdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"
    echo -e "${GREEN}✅ Database '$DB_NAME' created${NC}"
fi

# Create schema
run_sql "Creating database schema" "sql/schema.sql"

# Enable dblink for migration (if old database exists)
if [ "$OLD_DB_EXISTS" = true ]; then
    echo -e "${YELLOW}🔗 Enabling dblink for migration...${NC}"
    run_sql_cmd "Enabling dblink extension" "CREATE EXTENSION IF NOT EXISTS dblink;"
    
    echo -e "${YELLOW}📋 Ready to migrate data from $OLD_DB_NAME to $DB_NAME${NC}"
    echo -e "${YELLOW}⚠️  Migration requires manual execution of sql/migrate_from_rust_docs.sql${NC}"
    echo -e "${YELLOW}   Run: psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f sql/migrate_from_rust_docs.sql${NC}"
else
    echo -e "${YELLOW}ℹ️  Skipping migration setup (no source database found)${NC}"
fi

# Show database stats
echo -e "${GREEN}📊 Database setup summary:${NC}"
run_sql_cmd "Checking doc_type enum values" "SELECT unnest(enum_range(NULL::doc_type)) AS doc_types;"
run_sql_cmd "Checking table counts" "
SELECT 
    'documents' as table_name, 
    COUNT(*) as rows 
FROM documents 
UNION ALL 
SELECT 
    'document_sources' as table_name, 
    COUNT(*) as rows 
FROM document_sources;
"

echo -e "${GREEN}🎉 Database setup completed successfully!${NC}"
echo
echo -e "${YELLOW}Next steps:${NC}"
if [ "$OLD_DB_EXISTS" = true ]; then
    echo "1. Run migration: psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f sql/migrate_from_rust_docs.sql"
    echo "2. Verify migration results"
    echo "3. Update your .env file: DATABASE_URL=postgresql://$DB_USER:password@$DB_HOST:$DB_PORT/$DB_NAME"
else
    echo "1. Update your .env file: DATABASE_URL=postgresql://$DB_USER:password@$DB_HOST:$DB_PORT/$DB_NAME"
    echo "2. Start adding documentation sources"
fi
```

### scripts/stop.sh

```bash
#!/bin/bash

# Development environment stop script for Doc Server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🛑 Stopping Doc Server Development Environment${NC}"

# Stop development containers
echo -e "${YELLOW}📦 Stopping PostgreSQL database...${NC}"
docker compose -f docker-compose.dev.yml down

# Kill any running cargo processes
echo -e "${YELLOW}🔄 Stopping any running Rust processes...${NC}"
pkill -f "cargo run" 2>/dev/null || echo -e "${YELLOW}⚠️  No running cargo processes found${NC}"

echo -e "${GREEN}✅ Development environment stopped!${NC}"

# Optionally clean up volumes
read -p "Do you want to remove the database volume? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}🗑️  Removing database volume...${NC}"
    docker compose -f docker-compose.dev.yml down -v
    echo -e "${GREEN}✅ Database volume removed!${NC}"
fi
```

### clippy.toml

```toml
# Clippy configuration for Doc Server

# Cognitive complexity threshold
cognitive-complexity-threshold = 30

# Documentation requirements
missing-docs-in-crate-items = true

# Avoid false positives for some cases
avoid-breaking-exported-api = true

# Single letter variable names (allow common ones)
single-char-binding-names-threshold = 4

# Trivial copy types threshold
trivial-copy-size-limit = 32

# Type complexity threshold
type-complexity-threshold = 250

# Too many arguments threshold
too-many-arguments-threshold = 7

# Too many lines threshold
too-many-lines-threshold = 100

# Enum variant size threshold
enum-variant-size-threshold = 200
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  # PostgreSQL database with pgvector extension
  postgres:
    image: pgvector/pgvector:pg16
    container_name: doc-server-postgres
    environment:
      POSTGRES_DB: docs
      POSTGRES_USER: docserver
      POSTGRES_PASSWORD: development_password_change_in_production
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./sql/init:/docker-entrypoint-initdb.d
    networks:
      - doc-server-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U docserver -d docs"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Doc Server application
  doc-server:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: doc-server-app
    environment:
      DATABASE_URL: postgresql://docserver:development_password_change_in_production@postgres:5432/docs
      RUST_LOG: info,doc_server=debug
      PORT: 3001
      OPENAI_API_KEY: ${OPENAI_API_KEY:-dummy-key-for-testing}
    ports:
      - "3001:3001"
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - doc-server-network
    restart: unless-stopped

  # Redis for caching (optional future enhancement)
  redis:
    image: redis:7-alpine
    container_name: doc-server-redis
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    networks:
      - doc-server-network
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  postgres_data:
    driver: local
  redis_data:
    driver: local

networks:
  doc-server-network:
    driver: bridge
```

### sql/init/01-extensions.sql

```sql
-- Enable required PostgreSQL extensions for Doc Server
-- This script runs automatically when PostgreSQL starts in Docker

-- Enable pgvector for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Verify extensions are loaded
SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector', 'uuid-ossp');
```

### sql/init/02-setup-user-and-schema.sql

```sql
-- Setup user and schema for Doc Server
-- This script runs automatically when PostgreSQL starts in Docker

-- The user 'docserver' should already exist from POSTGRES_USER env var
-- The database 'docs' should already exist from POSTGRES_DB env var

-- Create the documents table with the harmonized schema
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type VARCHAR(50) NOT NULL CHECK (doc_type IN (
        'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
        'meteora', 'solana', 'ebpf', 'raydium', 'rust_best_practices'
    )),
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072), -- OpenAI text-embedding-3-large dimensions
    token_count INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure uniqueness per documentation type
    UNIQUE(doc_type, source_name, doc_path)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- Note: pgvector indexes (IVFFlat and HNSW) have a 2000 dimension limit
-- For 3072 dimensions (OpenAI text-embedding-3-large), we skip the index. 
-- Queries will still work but be slower. Consider upgrading pgvector 
-- or using 1536 dimensions if performance is critical.

-- Create a trigger to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Grant necessary permissions to the docserver user
GRANT ALL PRIVILEGES ON TABLE documents TO docserver;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO docserver;
```

### sql/migrate_from_rust_docs.sql

```sql
-- Migration script from rust_docs_vectors to new docs database
-- This script assumes you're connected to the NEW docs database
-- and have access to the old rust_docs_vectors database

-- Step 1: Migrate crate information to document_sources
INSERT INTO document_sources (
    doc_type, 
    source_name, 
    version, 
    config, 
    enabled, 
    last_updated, 
    total_docs, 
    total_tokens,
    created_at,
    updated_at
)
SELECT 
    'rust'::doc_type as doc_type,
    name as source_name,
    version,
    jsonb_build_object(
        'docs_rs_url', 'https://docs.rs/' || name || '/' || COALESCE(version, 'latest'),
        'migrated_from', 'rust_docs_vectors'
    ) as config,
    true as enabled,
    last_updated,
    total_docs,
    total_tokens,
    COALESCE(last_updated, CURRENT_TIMESTAMP) as created_at,
    CURRENT_TIMESTAMP as updated_at
FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT name, version, last_updated, total_docs, total_tokens FROM crates'
) AS old_crates(
    name VARCHAR(255),
    version VARCHAR(50),
    last_updated TIMESTAMP,
    total_docs INTEGER,
    total_tokens INTEGER
);

-- Step 2: Migrate document embeddings to documents table
INSERT INTO documents (
    doc_type,
    source_name,
    doc_path,
    content,
    metadata,
    embedding,
    token_count,
    created_at,
    updated_at
)
SELECT 
    'rust'::doc_type as doc_type,
    crate_name as source_name,
    doc_path,
    content,
    jsonb_build_object(
        'crate_name', crate_name,
        'migrated_from', 'rust_docs_vectors',
        'original_id', id
    ) as metadata,
    embedding,
    token_count,
    COALESCE(created_at, CURRENT_TIMESTAMP) as created_at,
    CURRENT_TIMESTAMP as updated_at
FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT id, crate_name, doc_path, content, embedding, token_count, created_at FROM doc_embeddings'
) AS old_docs(
    id INTEGER,
    crate_name VARCHAR(255),
    doc_path TEXT,
    content TEXT,
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMP
);

-- Step 3: Update document_sources statistics based on actual migrated data
UPDATE document_sources 
SET 
    total_docs = (
        SELECT COUNT(*) 
        FROM documents 
        WHERE documents.source_name = document_sources.source_name 
        AND documents.doc_type = document_sources.doc_type
    ),
    total_tokens = (
        SELECT COALESCE(SUM(token_count), 0) 
        FROM documents 
        WHERE documents.source_name = document_sources.source_name 
        AND documents.doc_type = document_sources.doc_type
    )
WHERE doc_type = 'rust';

-- Step 4: Verification queries (run these manually to verify migration)
/*
-- Verify crate count
SELECT 'document_sources' as table_name, COUNT(*) as count FROM document_sources WHERE doc_type = 'rust'
UNION ALL
SELECT 'original_crates' as table_name, COUNT(*) as count FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT COUNT(*) FROM crates'
) AS count_result(count BIGINT);

-- Verify document count  
SELECT 'documents' as table_name, COUNT(*) as count FROM documents WHERE doc_type = 'rust'
UNION ALL
SELECT 'original_doc_embeddings' as table_name, COUNT(*) as count FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT COUNT(*) FROM doc_embeddings'
) AS count_result(count BIGINT);

-- Verify sample data
SELECT source_name, COUNT(*) as doc_count, SUM(token_count) as total_tokens
FROM documents 
WHERE doc_type = 'rust' 
GROUP BY source_name 
ORDER BY doc_count DESC 
LIMIT 10;

-- Test vector search still works
SELECT source_name, doc_path, content
FROM documents 
WHERE doc_type = 'rust' 
AND embedding IS NOT NULL
ORDER BY embedding <-> (SELECT embedding FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL LIMIT 1)
LIMIT 5;
*/
```

### sql/schema.sql

```sql
-- Doc Server Database Schema
-- Harmonized schema supporting multiple documentation types

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create enum for documentation types
CREATE TYPE doc_type AS ENUM (
    'rust',
    'jupyter', 
    'birdeye',
    'cilium',
    'talos',
    'meteora',
    'raydium',
    'solana',
    'ebpf',
    'rust_best_practices'
);

-- Main documents table (replaces doc_embeddings)
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type doc_type NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name, doc_path)
);

-- Document sources configuration table (replaces crates)
CREATE TABLE document_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type doc_type NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    version VARCHAR(50),
    config JSONB DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_updated TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    total_docs INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Indexes for performance
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_created_at ON documents(created_at);
CREATE INDEX idx_documents_updated_at ON documents(updated_at);

-- Note: pgvector indexes (IVFFlat and HNSW) have a 2000 dimension limit
-- For 3072 dimensions (OpenAI text-embedding-3-large), we skip the index. 
-- Queries will still work but be slower. Consider upgrading pgvector 
-- or using 1536 dimensions if performance is critical.

-- Document sources indexes
CREATE INDEX idx_document_sources_doc_type ON document_sources(doc_type);
CREATE INDEX idx_document_sources_enabled ON document_sources(enabled);
CREATE INDEX idx_document_sources_last_updated ON document_sources(last_updated);

-- Trigger to update updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_document_sources_updated_at 
    BEFORE UPDATE ON document_sources 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Views for easier querying
CREATE VIEW rust_documents AS
SELECT * FROM documents WHERE doc_type = 'rust';

CREATE VIEW active_sources AS
SELECT * FROM document_sources WHERE enabled = true;

-- Function to get document stats by type
CREATE OR REPLACE FUNCTION get_doc_stats(doc_type_param doc_type)
RETURNS TABLE(
    source_name VARCHAR,
    doc_count BIGINT,
    total_tokens BIGINT,
    last_updated TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        ds.source_name,
        COUNT(d.id) as doc_count,
        COALESCE(SUM(d.token_count), 0) as total_tokens,
        MAX(d.updated_at) as last_updated
    FROM document_sources ds
    LEFT JOIN documents d ON ds.source_name = d.source_name AND ds.doc_type = d.doc_type
    WHERE ds.doc_type = doc_type_param
    GROUP BY ds.source_name
    ORDER BY doc_count DESC;
END;
$$ LANGUAGE plpgsql;
```

### sql/data/README.md

```markdown
# Database Dump and Restoration

This directory contains a complete dump of the Doc Server database with all ingested documentation.

## Database Contents

The `docs_database_dump.sql.gz` file contains:

- **40+ Rust crates** with full documentation and embeddings
- **BirdEye API documentation** (OpenAPI specs and endpoints)  
- **Solana documentation** (markdown, PDFs, architecture diagrams, ZK cryptography specs)
- **Vector embeddings** (3072-dimensional OpenAI text-embedding-3-large)
- **Complete metadata** for all document types

**Total size:** 67MB compressed (184MB uncompressed)
**Total documents:** 4,000+ with embeddings
**Documentation types:** rust, birdeye, solana

## Quick Restoration

### Option 1: Use Development Script (Recommended)
```bash
# This will automatically detect and load the database dump
./scripts/dev.sh --with-data
```

### Option 2: Manual Restoration to Docker Container
```bash
# Start PostgreSQL container
docker compose -f docker-compose.dev.yml up -d postgres

# Wait for it to be ready
sleep 5

# Restore the database
gunzip -c sql/data/docs_database_dump.sql.gz | \
  docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs
```

### Option 3: Manual Restoration to Local PostgreSQL
```bash
# If you have local PostgreSQL and want to restore there
gunzip -c sql/data/docs_database_dump.sql.gz | \
  psql -h localhost -p 5432 -U [your_username] -d docs
```

## Verification

After restoration, verify the data:

```bash
# Check document count
psql -c "SELECT doc_type, COUNT(*) FROM documents GROUP BY doc_type;" [connection_string]

# Test vector search
psql -c "SELECT COUNT(*) FROM documents WHERE embedding IS NOT NULL;" [connection_string]

# Sample query
psql -c "SELECT doc_type, source_name, LEFT(content, 100) FROM documents LIMIT 5;" [connection_string]
```

## Expected Results

You should see approximately:
- **3,000+ Rust documents** from 40+ crates
- **600+ BirdEye API endpoints** with OpenAPI documentation  
- **400+ Solana documents** including core docs, architecture diagrams, and ZK specs
- **100% embedding coverage** (all documents have vector embeddings)

## Regenerating the Dump

To create a fresh dump from your local database:

```bash
# Dump from local PostgreSQL
pg_dump -h localhost -p 5432 -U [username] -d docs > sql/data/docs_database_dump.sql

# Compress it
gzip sql/data/docs_database_dump.sql

# Or do both in one command
pg_dump -h localhost -p 5432 -U [username] -d docs | gzip > sql/data/docs_database_dump.sql.gz
```

## Notes

- The dump includes the complete schema (tables, indexes, extensions)
- pgvector extension is automatically included
- No need to run ingestion scripts if you restore this dump
- Embeddings are ready for immediate vector search
- All metadata and relationships are preserved
```

