//! Database models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use uuid::Uuid;

/// Document types supported by the system
/// Now using a newtype for dynamic configuration - any docType from tools.json is valid
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct DocType(pub String);

impl DocType {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for DocType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for DocType {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for DocType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Main document record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub doc_type: String,
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

/// Tool configuration from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    #[serde(rename = "docType")]
    pub doc_type: String,
    pub title: String,
    pub description: String,
    pub enabled: bool,
    /// Optional metadata hints for supported filters and content types
    #[serde(rename = "metadataHints", default)]
    pub metadata_hints: Option<ToolMetadataHints>,
}

/// Metadata hints for tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadataHints {
    /// Supported formats for filtering (e.g., `["markdown", "pdf", "bob", "msc"]`)
    #[serde(default)]
    pub supported_formats: Vec<String>,
    /// Supported complexity levels (e.g., `["beginner", "intermediate", "advanced"]`)
    #[serde(default)]
    pub supported_complexity_levels: Vec<String>,
    /// Supported categories (e.g., `["architecture", "api", "guides"]`)
    #[serde(default)]
    pub supported_categories: Vec<String>,
    /// Supported topics (e.g., `["consensus", "networking", "validators"]`)
    #[serde(default)]
    pub supported_topics: Vec<String>,
    /// Whether API version filtering is supported
    #[serde(default)]
    pub supports_api_version: bool,
}

/// Tools configuration container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub tools: Vec<ToolConfig>,
}

/// Job status enumeration for crate operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Crate job record for tracking background operations
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CrateJob {
    pub id: Uuid,
    pub crate_name: String,
    pub operation: String,
    pub status: JobStatus,
    pub progress: Option<i32>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Intelligent ingest job record for tracking asynchronous ingestion
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct IngestJob {
    pub id: Uuid,
    pub url: String,
    pub doc_type: String,
    pub status: JobStatus,
    pub started_at: chrono::DateTime<Utc>,
    pub finished_at: Option<chrono::DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Pagination parameters for listing operations
#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page: i32,
    pub limit: i32,
    pub offset: i32,
}

impl PaginationParams {
    /// Create new pagination parameters
    #[must_use]
    pub fn new(page: Option<i32>, limit: Option<i32>) -> Self {
        let page = page.unwrap_or(1).max(1);
        let limit = limit.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * limit;

        Self {
            page,
            limit,
            offset,
        }
    }
}

/// Paginated response container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: i32,
    pub total_pages: i32,
    pub total_items: i64,
    pub has_previous: bool,
    pub has_next: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    #[must_use]
    pub fn new(items: Vec<T>, pagination: &PaginationParams, total_items: i64) -> Self {
        // Compute total pages using integer math to avoid float casts
        // Ceiling division: (total_items + limit - 1) / limit
        let limit_i64 = i64::from(pagination.limit);
        let total_pages_i64 = if limit_i64 > 0 {
            (total_items + (limit_i64 - 1)).div_euclid(limit_i64)
        } else {
            0
        };
        let total_pages = i32::try_from(total_pages_i64).unwrap_or(i32::MAX);
        let has_previous = pagination.page > 1;
        let has_next = pagination.page < total_pages;

        Self {
            items,
            page: pagination.page,
            total_pages,
            total_items,
            has_previous,
            has_next,
        }
    }
}

/// Crate information derived from document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub documentation_url: Option<String>,
    pub total_docs: i32,
    pub total_tokens: i64,
    pub last_updated: DateTime<Utc>,
}

/// Crate statistics for system monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateStatistics {
    pub total_crates: i64,
    pub active_crates: i64,
    pub total_docs_managed: i64,
    pub total_tokens_managed: i64,
    pub average_docs_per_crate: f64,
    pub last_update: Option<DateTime<Utc>>,
}
