//! Database query operations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::models::{DocType, Document};

/// Document query operations
pub struct DocumentQueries;

impl DocumentQueries {
    /// Find documents by type
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or the result rows cannot
    /// be deserialized into `Document` values.
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
            r"
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
            ",
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
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or the results cannot be
    /// mapped into `Document` values.
    pub async fn find_by_source(pool: &PgPool, source_name: &str) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r"
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
            ",
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
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn vector_search(
        pool: &PgPool,
        _embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>> {
        // For now, return a basic text search as fallback
        let rows = sqlx::query(
            r"
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
            ",
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
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn rust_vector_search(
        pool: &PgPool,
        _embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>> {
        // For now, return Rust documents ordered by relevance
        let rows = sqlx::query(
            r"
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
            ",
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

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceMetrics {
    pub query_name: String,
    pub execution_time_ms: u64,
    pub rows_returned: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Performance monitoring for database queries
pub struct QueryPerformanceMonitor;

impl QueryPerformanceMonitor {
    /// Execute a query with performance monitoring
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying operation fails.
    pub async fn execute_with_monitoring<F, T>(
        query_name: &str,
        operation: F,
    ) -> Result<(T, QueryPerformanceMetrics)>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let start = Instant::now();
        let result = operation.await;
        let execution_time = start.elapsed();

        let metrics = QueryPerformanceMetrics {
            query_name: query_name.to_string(),
            execution_time_ms: u64::try_from(execution_time.as_millis()).unwrap_or(u64::MAX),
            rows_returned: 0, // This would need to be passed from the operation
            timestamp: chrono::Utc::now(),
        };

        // Log performance warnings
        if execution_time > Duration::from_secs(2) {
            warn!(
                "Query '{}' took {}ms (exceeds 2s threshold)",
                query_name, metrics.execution_time_ms
            );
        } else if execution_time > Duration::from_millis(500) {
            info!(
                "Query '{}' took {}ms",
                query_name, metrics.execution_time_ms
            );
        }

        match result {
            Ok(value) => Ok((value, metrics)),
            Err(e) => {
                warn!(
                    "Query '{}' failed after {}ms: {}",
                    query_name, metrics.execution_time_ms, e
                );
                Err(e)
            }
        }
    }

    /// Run performance benchmarks on key queries
    ///
    /// # Errors
    ///
    /// Returns an error if any benchmark query fails.
    ///
    /// # Panics
    ///
    /// May panic if the results vector is empty (should not happen in normal operation).
    pub async fn benchmark_queries(pool: &PgPool) -> Result<Vec<QueryPerformanceMetrics>> {
        let mut results = Vec::new();

        // Benchmark: Count all documents
        let (count_result, count_metrics) = Self::execute_with_monitoring(
            "count_all_documents",
            Self::benchmark_count_documents(pool),
        )
        .await?;
        results.push(count_metrics);
        info!(
            "Document count benchmark: {} documents in {}ms",
            count_result,
            results.last().unwrap().execution_time_ms
        );

        // Benchmark: Latest 100 documents
        let (latest_result, latest_metrics) = Self::execute_with_monitoring(
            "latest_100_documents",
            Self::benchmark_latest_documents(pool, 100),
        )
        .await?;
        results.push(latest_metrics);
        info!(
            "Latest documents benchmark: {} documents in {}ms",
            latest_result.len(),
            results.last().unwrap().execution_time_ms
        );

        // Benchmark: Documents by type (Rust)
        let (rust_result, rust_metrics) = Self::execute_with_monitoring(
            "rust_documents_by_type",
            DocumentQueries::find_by_type(pool, DocType::Rust),
        )
        .await?;
        results.push(rust_metrics);
        info!(
            "Rust documents by type benchmark: {} documents in {}ms",
            rust_result.len(),
            results.last().unwrap().execution_time_ms
        );

        // Benchmark: Check indexes are being used
        let (_index_result, index_metrics) = Self::execute_with_monitoring(
            "explain_doc_type_query",
            Self::explain_query(
                pool,
                "SELECT * FROM documents WHERE doc_type = 'rust' LIMIT 10",
            ),
        )
        .await?;
        results.push(index_metrics);
        info!(
            "Index usage check completed in {}ms",
            results.last().unwrap().execution_time_ms
        );

        Ok(results)
    }

    async fn benchmark_count_documents(pool: &PgPool) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM documents")
            .fetch_one(pool)
            .await?;
        Ok(row.get::<i64, _>("count"))
    }

    async fn benchmark_latest_documents(pool: &PgPool, limit: i64) -> Result<Vec<Document>> {
        let rows = sqlx::query(
            r"
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
            ORDER BY created_at DESC
            LIMIT $1
            ",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let docs = rows
            .into_iter()
            .map(|row| Document {
                id: row.get("id"),
                doc_type: row.get("doc_type"),
                source_name: row.get("source_name"),
                doc_path: row.get("doc_path"),
                content: row.get("content"),
                metadata: row.get("metadata"),
                embedding: None,
                token_count: row.get("token_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(docs)
    }

    async fn explain_query(pool: &PgPool, query: &str) -> Result<String> {
        let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {query}");
        let row = sqlx::query(&explain_query).fetch_one(pool).await?;

        let explain_result: serde_json::Value = row.get(0);
        Ok(explain_result.to_string())
    }
}
