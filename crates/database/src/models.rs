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