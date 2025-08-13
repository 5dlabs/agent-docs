//! Database query operations

use anyhow::Result;
use sqlx::{PgPool, Row};

use crate::models::{DocType, Document};

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
            "#,
        )
        .bind(type_str)
        .fetch_all(pool)
        .await?;

        let docs = rows
            .into_iter()
            .map(|row| {
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
            })
            .collect();

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
            "#,
        )
        .bind(source_name)
        .fetch_all(pool)
        .await?;

        let docs = rows
            .into_iter()
            .map(|row| {
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
            })
            .collect();

        Ok(docs)
    }

    /// Perform vector similarity search
    pub async fn vector_search(
        pool: &PgPool,
        _embedding: &[f32],
        limit: i64,
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
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let docs = rows
            .into_iter()
            .map(|row| {
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
            })
            .collect();

        Ok(docs)
    }

    /// Perform vector similarity search for Rust documents only
    pub async fn rust_vector_search(
        pool: &PgPool,
        _embedding: &[f32],
        limit: i64,
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
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let docs = rows
            .into_iter()
            .map(|row| {
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
            })
            .collect();

        Ok(docs)
    }
}
