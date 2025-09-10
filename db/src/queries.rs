//! Database query operations

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::models::{DocType, Document};

/// Metadata filters for document search
#[derive(Debug, Clone, Default)]
pub struct MetadataFilters {
    pub format: Option<String>,
    pub complexity: Option<String>,
    pub category: Option<String>,
    pub topic: Option<String>,
    pub api_version: Option<String>,
}

/// Trait for types that can report how many rows they represent
pub trait RowCountable {
    fn row_count(&self) -> usize;
}

/// Implementation for vectors (collections of items)
impl<T> RowCountable for Vec<T> {
    fn row_count(&self) -> usize {
        self.len()
    }
}

/// Implementation for single numeric results (like COUNT queries)
impl RowCountable for i64 {
    fn row_count(&self) -> usize {
        // For count queries, the result represents the count itself
        // Convert to usize, clamping to prevent overflow
        (*self).try_into().unwrap_or(0)
    }
}

/// Implementation for string results (like EXPLAIN queries)
impl RowCountable for String {
    fn row_count(&self) -> usize {
        // For string results like EXPLAIN, we count this as 1 row
        1
    }
}

/// Document query operations
pub struct DocumentQueries;

impl DocumentQueries {
    async fn is_doc_type_enum(pool: &PgPool, table: &str) -> Result<bool> {
        // Detect if the doc_type column is a USER-DEFINED type (enum) for the given table
        let row = sqlx::query(
            r"
            SELECT data_type = 'USER-DEFINED' AS is_enum
            FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1 AND column_name = 'doc_type'
            LIMIT 1
            ",
        )
        .bind(table)
        .fetch_optional(pool)
        .await?;

        Ok(row
            .and_then(|r| r.try_get::<bool, _>("is_enum").ok())
            .unwrap_or(false))
    }
    /// Ensure document source exists
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn ensure_document_source(
        pool: &PgPool,
        doc_type: &str,
        source_name: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO document_sources (doc_type, source_name, config, enabled)
            VALUES ($1, $2, '{"auto_created": true}', true)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(doc_type)
        .bind(source_name)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Insert a single document
    ///
    /// # Errors
    ///
    /// Returns an error if the database insertion fails.
    #[allow(clippy::too_many_lines)]
    pub async fn insert_document(
        pool: &PgPool,
        document: &crate::models::Document,
    ) -> Result<crate::models::Document> {
        let row = sqlx::query(
            r"
            INSERT INTO documents (
                id,
                doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            ON CONFLICT (id) DO UPDATE SET
                content = EXCLUDED.content,
                metadata = EXCLUDED.metadata,
                token_count = EXCLUDED.token_count,
                updated_at = EXCLUDED.updated_at
            RETURNING
                id,
                doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at
            ",
        )
        .bind(document.id)
        .bind(&document.doc_type)
        .bind(&document.source_name)
        .bind(&document.doc_path)
        .bind(&document.content)
        .bind(&document.metadata)
        .bind(document.token_count)
        .bind(document.created_at.unwrap_or_else(chrono::Utc::now))
        .fetch_one(pool)
        .await?;

        let doc = crate::models::Document {
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
        };

        Ok(doc)
    }

    /// Batch insert multiple documents with transaction support
    ///
    /// # Errors
    ///
    /// Returns an error if the database batch insertion fails.
    #[allow(clippy::too_many_lines)]
    pub async fn batch_insert_documents(
        pool: &PgPool,
        documents: &[crate::models::Document],
    ) -> Result<Vec<crate::models::Document>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        // Ensure document sources exist for all documents
        let mut sources_to_create = std::collections::HashSet::new();
        for doc in documents {
            sources_to_create.insert((doc.doc_type.clone(), doc.source_name.clone()));
        }

        for (doc_type, source_name) in sources_to_create {
            Self::ensure_document_source(pool, doc_type.as_str(), &source_name).await?;
        }

        let mut transaction = pool.begin().await?;
        let mut inserted_docs = Vec::new();

        for doc in documents {
            let row = sqlx::query(
                r"
                INSERT INTO documents (
                    id,
                    doc_type,
                    source_name,
                    doc_path,
                    content,
                    metadata,
                    token_count,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    metadata = EXCLUDED.metadata,
                    token_count = EXCLUDED.token_count,
                    updated_at = EXCLUDED.updated_at

                RETURNING
                    id,
                    doc_type,
                    source_name,
                    doc_path,
                    content,
                    metadata,
                    token_count,
                    created_at,
                    updated_at
                ",
            )
            .bind(doc.id)
            .bind(&doc.doc_type)
            .bind(&doc.source_name)
            .bind(&doc.doc_path)
            .bind(&doc.content)
            .bind(&doc.metadata)
            .bind(doc.token_count)
            .bind(doc.created_at.unwrap_or_else(chrono::Utc::now))
            .fetch_one(&mut *transaction)
            .await?;

            let inserted_doc = crate::models::Document {
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
            };

            inserted_docs.push(inserted_doc);
        }

        transaction.commit().await?;
        Ok(inserted_docs)
    }

    /// Delete documents by source name
    ///
    /// # Errors
    ///
    /// Returns an error if the database deletion fails.
    pub async fn delete_by_source(pool: &PgPool, source_name: &str) -> Result<i64> {
        let result = sqlx::query("DELETE FROM documents WHERE source_name = $1")
            .bind(source_name)
            .execute(pool)
            .await?;

        Ok(result.rows_affected().try_into().unwrap_or(i64::MAX))
    }

    /// Find documents by type
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or the result rows cannot
    /// be deserialized into `Document` values.
    pub async fn find_by_type(pool: &PgPool, doc_type: DocType) -> Result<Vec<Document>> {
        // Delegate to string-based variant to avoid hardcoding mappings here
        Self::find_by_type_str(pool, &doc_type.to_string()).await
    }

    /// Find documents by type (string-based, supports config-driven docType values)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails or the result rows cannot
    /// be deserialized into `Document` values.
    pub async fn find_by_type_str(pool: &PgPool, doc_type: &str) -> Result<Vec<Document>> {
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
        .bind(doc_type)
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
        query: &str,
        _embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>> {
        // Perform full-text search on Rust documents with relevance ranking
        // Try full-text search first, fallback to tokenized ILIKE if FTS not available
        let fts_sql = r"
            SELECT
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at,
                ts_rank_cd(
                    to_tsvector('english', coalesce(content,'')),
                    websearch_to_tsquery('english', $1)
                ) AS rank
            FROM documents
            WHERE doc_type::text = 'rust'
              AND (
                    to_tsvector('english', coalesce(content,'')) @@ websearch_to_tsquery('english', $1)
                 OR doc_path ILIKE $2
                 OR content ILIKE $2
              )
            ORDER BY
              rank DESC,
              created_at DESC
            LIMIT $3
        ";

        let fts_attempt = sqlx::query(fts_sql)
            .bind(query)
            .bind(format!("%{query}%"))
            .bind(limit)
            .fetch_all(pool)
            .await;

        let rows = if let Ok(rows) = fts_attempt {
            rows
        } else {
            // Fallback: tokenized ILIKE requiring all significant tokens
            let tokens: Vec<String> = query
                .split_whitespace()
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
                .filter(|t| t.len() >= 3)
                .map(|t| format!("%{t}%"))
                .collect();

            let mut where_parts = vec!["doc_type::text = $1".to_string()];
            let mut binds: Vec<String> = Vec::new();
            let mut bind_index = 2;
            for _tok in &tokens {
                where_parts.push(format!(
                    "(content ILIKE ${bind_index} OR doc_path ILIKE ${bind_index})"
                ));
                bind_index += 1;
            }
            // Re-add binds in order matching the placeholders
            binds.extend(tokens.clone());

            // If no tokens, fall back to simple ILIKE of full query
            if tokens.is_empty() {
                where_parts.push("(content ILIKE $2 OR doc_path ILIKE $2)".to_string());
                binds.push(format!("%{query}%"));
                bind_index = 3;
            }

            let sql = format!(
                    "SELECT id, doc_type::text as doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at \
                     FROM documents WHERE {} ORDER BY created_at DESC LIMIT ${}",
                    where_parts.join(" AND "),
                    bind_index
                );

            let mut q = sqlx::query(&sql).bind("rust");
            for b in &binds {
                q = q.bind(b);
            }
            q = q.bind(limit);
            q.fetch_all(pool).await?
        };

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

    /// Perform vector similarity search for documents of a specific type
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn doc_type_vector_search(
        pool: &PgPool,
        doc_type: &str,
        query: &str,
        _embedding: &[f32],
        limit: i64,
    ) -> Result<Vec<Document>> {
        // Attempt full-text search first (uses built-in FTS, no extension required)
        // Fallback to tokenized ILIKE if FTS functions are unavailable
        let fts_sql = r"
            SELECT
                id,
                doc_type::text as doc_type,
                source_name,
                doc_path,
                content,
                metadata,
                token_count,
                created_at,
                updated_at,
                ts_rank_cd(
                    to_tsvector('english', coalesce(content,'')),
                    websearch_to_tsquery('english', $2)
                ) AS rank
            FROM documents
            WHERE doc_type::text = $1
              AND (
                    to_tsvector('english', coalesce(content,'')) @@ websearch_to_tsquery('english', $2)
                 OR doc_path ILIKE $3
              )
            ORDER BY 
              rank DESC,
              created_at DESC
            LIMIT $4
        ";

        let fts_attempt = sqlx::query(fts_sql)
            .bind(doc_type)
            .bind(query)
            .bind(format!("%{query}%"))
            .bind(limit)
            .fetch_all(pool)
            .await;

        let rows = if let Ok(rows) = fts_attempt {
            rows
        } else {
            // Fallback: tokenized ILIKE requiring all significant tokens
            let tokens: Vec<String> = query
                .split_whitespace()
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
                .filter(|t| t.len() >= 3)
                .map(|t| format!("%{t}%"))
                .collect();

            let mut where_parts = vec!["doc_type::text = $1".to_string()];
            let mut binds: Vec<String> = Vec::new();
            let mut bind_index = 2;
            for _tok in &tokens {
                where_parts.push(format!(
                    "(content ILIKE ${bind_index} OR doc_path ILIKE ${bind_index})"
                ));
                bind_index += 1;
                // push corresponding bind after the loop using same index order
            }
            // Re-add binds in order matching the placeholders constructed above
            binds.extend(tokens.clone());
            // If no tokens, fall back to simple ILIKE of full query
            if tokens.is_empty() {
                where_parts.push("(content ILIKE $2 OR doc_path ILIKE $2)".to_string());
                binds.push(format!("%{query}%"));
                bind_index = 3;
            }

            let sql = format!(
                    "SELECT id, doc_type::text as doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at \
                     FROM documents WHERE {} ORDER BY created_at DESC LIMIT ${}",
                    where_parts.join(" AND "),
                    bind_index
                );
            let mut q = sqlx::query(&sql).bind(doc_type);
            for b in &binds {
                q = q.bind(b);
            }
            q = q.bind(limit);
            q.fetch_all(pool).await?
        };

        info!(
            "doc_type_vector_search: Found {} documents for doc_type '{}'",
            rows.len(),
            doc_type
        );

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

    /// Perform vector similarity search with metadata filtering
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    #[allow(clippy::too_many_lines)]
    pub async fn doc_type_vector_search_with_filters(
        pool: &PgPool,
        doc_type: &str,
        query: &str,
        _embedding: &[f32],
        limit: i64,
        filters: &MetadataFilters,
    ) -> Result<Vec<Document>> {
        // Try FTS variant with ranking and metadata filters
        let mut where_parts = vec!["doc_type::text = $1".to_string()];
        // FTS predicate and doc_path fallback
        where_parts.push("(to_tsvector('english', coalesce(content,'')) @@ websearch_to_tsquery('english', $2) OR doc_path ILIKE $3)".to_string());
        let mut bind_index = 4;
        if filters.format.is_some() {
            where_parts.push(format!("(metadata->>'format' = ${bind_index})"));
            bind_index += 1;
        }
        if filters.complexity.is_some() {
            where_parts.push(format!("(metadata->>'complexity' = ${bind_index})"));
            bind_index += 1;
        }
        if filters.category.is_some() {
            where_parts.push(format!("(metadata->>'category' = ${bind_index})"));
            bind_index += 1;
        }
        if filters.topic.is_some() {
            where_parts.push(format!("(metadata->>'topic' = ${bind_index})"));
            bind_index += 1;
        }
        if filters.api_version.is_some() {
            where_parts.push(format!("(metadata->>'api_version' = ${bind_index})"));
            bind_index += 1;
        }

        let fts_sql = format!(
            "SELECT id, doc_type::text as doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at, \
             ts_rank_cd(to_tsvector('english', coalesce(content,'')), websearch_to_tsquery('english', $2)) AS rank \
             FROM documents WHERE {} ORDER BY rank DESC, created_at DESC LIMIT ${}",
            where_parts.join(" AND "),
            bind_index
        );

        let mut q = sqlx::query(&fts_sql)
            .bind(doc_type)
            .bind(query)
            .bind(format!("%{query}%"));
        if let Some(v) = &filters.format {
            q = q.bind(v);
        }
        if let Some(v) = &filters.complexity {
            q = q.bind(v);
        }
        if let Some(v) = &filters.category {
            q = q.bind(v);
        }
        if let Some(v) = &filters.topic {
            q = q.bind(v);
        }
        if let Some(v) = &filters.api_version {
            q = q.bind(v);
        }
        q = q.bind(limit);

        let rows = match q.fetch_all(pool).await {
            Ok(rows) => rows,
            Err(e) => {
                // Log the error before falling back
                tracing::warn!(
                    "FTS query with filters failed: {}, falling back to ILIKE",
                    e
                );
                // Fallback to tokenized ILIKE with filters
                let tokens: Vec<String> = query
                    .split_whitespace()
                    .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
                    .filter(|t| t.len() >= 3)
                    .map(|t| format!("%{t}%"))
                    .collect();

                let mut parts = vec!["doc_type::text = $1".to_string()];
                let mut idx = 2;
                for _t in &tokens {
                    parts.push(format!("(content ILIKE ${idx} OR doc_path ILIKE ${idx})"));
                    idx += 1;
                }
                if tokens.is_empty() {
                    parts.push("(content ILIKE $2 OR doc_path ILIKE $2)".to_string());
                    idx = 3;
                }
                if filters.format.is_some() {
                    parts.push(format!("(metadata->>'format' = ${idx})"));
                    idx += 1;
                }
                if filters.complexity.is_some() {
                    parts.push(format!("(metadata->>'complexity' = ${idx})"));
                    idx += 1;
                }
                if filters.category.is_some() {
                    parts.push(format!("(metadata->>'category' = ${idx})"));
                    idx += 1;
                }
                if filters.topic.is_some() {
                    parts.push(format!("(metadata->>'topic' = ${idx})"));
                    idx += 1;
                }
                if filters.api_version.is_some() {
                    parts.push(format!("(metadata->>'api_version' = ${idx})"));
                    idx += 1;
                }
                let sql = format!(
                    "SELECT id, doc_type::text as doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at \
                     FROM documents WHERE {} ORDER BY created_at DESC LIMIT ${}",
                    parts.join(" AND "),
                    idx
                );
                let mut q2 = sqlx::query(&sql).bind(doc_type);
                if tokens.is_empty() {
                    q2 = q2.bind(format!("%{query}%"));
                } else {
                    for t in &tokens {
                        q2 = q2.bind(t);
                    }
                }
                if let Some(v) = &filters.format {
                    q2 = q2.bind(v);
                }
                if let Some(v) = &filters.complexity {
                    q2 = q2.bind(v);
                }
                if let Some(v) = &filters.category {
                    q2 = q2.bind(v);
                }
                if let Some(v) = &filters.topic {
                    q2 = q2.bind(v);
                }
                if let Some(v) = &filters.api_version {
                    q2 = q2.bind(v);
                }
                q2 = q2.bind(limit);
                match q2.fetch_all(pool).await {
                    Ok(rows) => rows,
                    Err(e) => {
                        tracing::error!(
                            "Both FTS and ILIKE queries failed with filters. Error: {}",
                            e
                        );
                        return Err(e.into());
                    }
                }
            }
        };

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
        T: RowCountable,
    {
        let start = Instant::now();
        let result = operation.await;
        let execution_time = start.elapsed();

        match result {
            Ok(value) => {
                let row_count = value.row_count();
                let metrics = QueryPerformanceMetrics {
                    query_name: query_name.to_string(),
                    execution_time_ms: u64::try_from(execution_time.as_millis())
                        .unwrap_or(u64::MAX),
                    rows_returned: row_count,
                    timestamp: chrono::Utc::now(),
                };

                // Log performance warnings
                if execution_time > Duration::from_secs(2) {
                    warn!(
                        "Query '{}' took {}ms (exceeds 2s threshold), returned {} rows",
                        query_name, metrics.execution_time_ms, row_count
                    );
                } else if execution_time > Duration::from_millis(500) {
                    info!(
                        "Query '{}' took {}ms, returned {} rows",
                        query_name, metrics.execution_time_ms, row_count
                    );
                }

                Ok((value, metrics))
            }
            Err(e) => {
                let metrics = QueryPerformanceMetrics {
                    query_name: query_name.to_string(),
                    execution_time_ms: u64::try_from(execution_time.as_millis())
                        .unwrap_or(u64::MAX),
                    rows_returned: 0, // No rows returned on error
                    timestamp: chrono::Utc::now(),
                };

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
            DocumentQueries::find_by_type(pool, DocType::from("rust")),
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
                "SELECT * FROM documents WHERE doc_type::text = 'rust' LIMIT 10",
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

/// Crate job query operations
pub struct CrateJobQueries;

impl CrateJobQueries {
    /// Create a new crate job
    ///
    /// # Errors
    ///
    /// Returns an error if the database insertion fails.
    pub async fn create_job(
        pool: &PgPool,
        crate_name: &str,
        operation: &str,
    ) -> Result<crate::models::CrateJob> {
        let job_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        let row = sqlx::query_as::<_, crate::models::CrateJob>(
            r"
            INSERT INTO crate_jobs (id, crate_name, operation, status, started_at, created_at, updated_at)
            VALUES ($1, $2, $3, 'queued', $4, $4, $4)
            RETURNING *
            "
        )
        .bind(job_id)
        .bind(crate_name)
        .bind(operation)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Find job by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn find_job_by_id(
        pool: &PgPool,
        job_id: uuid::Uuid,
    ) -> Result<Option<crate::models::CrateJob>> {
        let row =
            sqlx::query_as::<_, crate::models::CrateJob>("SELECT * FROM crate_jobs WHERE id = $1")
                .bind(job_id)
                .fetch_optional(pool)
                .await?;

        Ok(row)
    }

    /// Update job status
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub async fn update_job_status(
        pool: &PgPool,
        job_id: uuid::Uuid,
        status: crate::models::JobStatus,
        progress: Option<i32>,
        error: Option<&str>,
    ) -> Result<crate::models::CrateJob> {
        let now = chrono::Utc::now();
        let finished_at = if matches!(
            status,
            crate::models::JobStatus::Completed
                | crate::models::JobStatus::Failed
                | crate::models::JobStatus::Cancelled
        ) {
            Some(now)
        } else {
            None
        };

        let row = sqlx::query_as::<_, crate::models::CrateJob>(
            r"
            UPDATE crate_jobs
            SET status = $2,
                progress = $3,
                error = $4,
                finished_at = $5,
                -- Set started_at when transitioning into running for the first time
                started_at = CASE
                    WHEN $2 = 'running'::job_status AND status <> 'running'::job_status THEN $6
                    ELSE started_at
                END,
                updated_at = $6
            WHERE id = $1
            RETURNING *
            ",
        )
        .bind(job_id)
        .bind(status)
        .bind(progress)
        .bind(error)
        .bind(finished_at)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Find active jobs (queued or running)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn find_active_jobs(pool: &PgPool) -> Result<Vec<crate::models::CrateJob>> {
        let rows = sqlx::query_as::<_, crate::models::CrateJob>(
            r"
            SELECT * FROM crate_jobs 
            WHERE status IN ('queued', 'running')
            ORDER BY created_at ASC
            ",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Clean up old completed jobs
    ///
    /// # Errors
    ///
    /// Returns an error if the cleanup operation fails.
    pub async fn cleanup_old_jobs(pool: &PgPool) -> Result<i32> {
        let result = sqlx::query(
            r"
            DELETE FROM crate_jobs 
            WHERE status IN ('completed', 'failed', 'cancelled') 
            AND finished_at < CURRENT_TIMESTAMP - INTERVAL '30 days'
            ",
        )
        .execute(pool)
        .await?;

        Ok(i32::try_from(result.rows_affected()).unwrap_or(i32::MAX))
    }
}

/// Ingest job query operations
pub struct IngestJobQueries;

impl IngestJobQueries {
    /// Create a new ingest job
    ///
    /// # Errors
    ///
    /// Returns an error if the database insertion fails.
    pub async fn create_job(
        pool: &PgPool,
        url: &str,
        doc_type: &str,
    ) -> Result<crate::models::IngestJob> {
        let job_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        let row = sqlx::query_as::<_, crate::models::IngestJob>(
            r"
            INSERT INTO ingest_jobs (
                id, url, doc_type, status, started_at, created_at, updated_at
            ) VALUES ($1, $2, $3, 'queued', $4, $4, $4)
            RETURNING *
            ",
        )
        .bind(job_id)
        .bind(url)
        .bind(doc_type)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Find ingest job by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn find_job_by_id(
        pool: &PgPool,
        job_id: uuid::Uuid,
    ) -> Result<Option<crate::models::IngestJob>> {
        let row = sqlx::query_as::<_, crate::models::IngestJob>(
            "SELECT * FROM ingest_jobs WHERE id = $1",
        )
        .bind(job_id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Update ingest job status
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub async fn update_job_status(
        pool: &PgPool,
        job_id: uuid::Uuid,
        status: crate::models::JobStatus,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<crate::models::IngestJob> {
        let now = chrono::Utc::now();
        let finished_at = if matches!(
            status,
            crate::models::JobStatus::Completed
                | crate::models::JobStatus::Failed
                | crate::models::JobStatus::Cancelled
        ) {
            Some(now)
        } else {
            None
        };

        let row = sqlx::query_as::<_, crate::models::IngestJob>(
            r"
            UPDATE ingest_jobs
            SET status = $2,
                output = COALESCE($3, output),
                error = COALESCE($4, error),
                finished_at = COALESCE($5, finished_at),
                -- Set started_at when transitioning into running for the first time
                started_at = CASE
                    WHEN $2 = 'running'::job_status AND status <> 'running'::job_status THEN $6
                    ELSE started_at
                END,
                updated_at = $6
            WHERE id = $1
            RETURNING *
            ",
        )
        .bind(job_id)
        .bind(status)
        .bind(output)
        .bind(error)
        .bind(finished_at)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Clean up old ingest jobs (older than 30 days and completed/failed/cancelled)
    ///
    /// # Errors
    ///
    /// Returns an error if the cleanup operation fails.
    pub async fn cleanup_old_jobs(pool: &PgPool) -> Result<i32> {
        let result = sqlx::query(
            r"
            DELETE FROM ingest_jobs
            WHERE status IN ('completed', 'failed', 'cancelled')
              AND finished_at < CURRENT_TIMESTAMP - INTERVAL '30 days'
            ",
        )
        .execute(pool)
        .await?;

        Ok(i32::try_from(result.rows_affected()).unwrap_or(i32::MAX))
    }
}

/// Crate-related query operations (using documents table only)
pub struct CrateQueries;

impl CrateQueries {
    /// Get list of crates from document metadata with pagination
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    #[allow(clippy::too_many_lines)]
    pub async fn list_crates(
        pool: &PgPool,
        pagination: &crate::models::PaginationParams,
        name_pattern: Option<&str>,
    ) -> Result<crate::models::PaginatedResponse<crate::models::CrateInfo>> {
        // Build the base query for crate information from documents
        let mut query_parts = vec![r"
            WITH crate_stats AS (
                SELECT 
                    metadata->>'crate_name' as crate_name,
                    metadata->>'crate_version' as crate_version,
                    COUNT(*) as total_docs,
                    COALESCE(SUM(token_count), 0) as total_tokens,
                    MAX(created_at) as last_updated
                FROM documents 
                WHERE doc_type::text = 'rust' 
                AND metadata->>'crate_name' IS NOT NULL
            "
        .to_string()];

        // Add name pattern filter if provided
        if name_pattern.is_some() {
            query_parts.push("AND metadata->>'crate_name' ILIKE $3".to_string());
        }

        query_parts.push(
            r"
                GROUP BY metadata->>'crate_name', metadata->>'crate_version'
            )
            SELECT 
                COALESCE(crate_name, 'unknown') as name,
                COALESCE(crate_version, 'latest') as version,
                '' as description,
                '' as documentation_url,
                total_docs::int as total_docs,
                total_tokens,
                last_updated
            FROM crate_stats
            WHERE crate_name IS NOT NULL
            ORDER BY crate_name
            LIMIT $1 OFFSET $2
        "
            .to_string(),
        );

        let query_str = query_parts.join(" ");

        // Execute main query
        let mut query = sqlx::query(&query_str)
            .bind(pagination.limit)
            .bind(pagination.offset);

        if let Some(pattern) = name_pattern {
            query = query.bind(format!("%{pattern}%"));
        }

        let rows = query.fetch_all(pool).await?;

        // Get total count
        let mut count_query_parts = vec![r"
            SELECT COUNT(DISTINCT metadata->>'crate_name') 
            FROM documents 
            WHERE doc_type::text = 'rust' 
            AND metadata->>'crate_name' IS NOT NULL
            "
        .to_string()];

        if name_pattern.is_some() {
            count_query_parts.push("AND metadata->>'crate_name' ILIKE $1".to_string());
        }

        let count_query_str = count_query_parts.join(" ");
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_query_str);

        if let Some(pattern) = name_pattern {
            count_query = count_query.bind(format!("%{pattern}%"));
        }

        let total_items = count_query.fetch_one(pool).await?;

        // Convert rows to CrateInfo
        let items = rows
            .into_iter()
            .map(|row| {
                let name: String = row.get("name");
                let version: String = row.get("version");
                let description: String = row.get("description");
                let documentation_url: String = row.get("documentation_url");
                let total_docs: i32 = row.get("total_docs");
                let total_tokens: i64 = row.get("total_tokens");
                let last_updated: DateTime<Utc> = row.get("last_updated");

                crate::models::CrateInfo {
                    name,
                    version,
                    description: if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    documentation_url: if documentation_url.is_empty() {
                        None
                    } else {
                        Some(documentation_url)
                    },
                    total_docs,
                    total_tokens,
                    last_updated,
                }
            })
            .collect();

        Ok(crate::models::PaginatedResponse::new(
            items,
            pagination,
            total_items,
        ))
    }

    /// Get crate statistics
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_crate_statistics(pool: &PgPool) -> Result<crate::models::CrateStatistics> {
        let row = sqlx::query(
            r"
            WITH crate_stats AS (
                SELECT 
                    metadata->>'crate_name' as crate_name,
                    COUNT(*) as docs_count,
                    COALESCE(SUM(CAST(token_count AS BIGINT)), 0) as tokens_count,
                    MAX(created_at) as last_updated
                FROM documents 
                WHERE doc_type::text = 'rust' 
                AND metadata->>'crate_name' IS NOT NULL
                GROUP BY metadata->>'crate_name'
            )
            SELECT 
                COUNT(*)::bigint as total_crates,
                COUNT(*)::bigint as active_crates,
                COALESCE(SUM(docs_count), 0)::bigint as total_docs,
                MAX(last_updated) as last_update
            FROM crate_stats
            ",
        )
        .fetch_one(pool)
        .await?;

        let total_crates: i64 = row.get("total_crates");
        let active_crates: i64 = row.get("active_crates");
        let total_docs: i64 = row.get("total_docs");
        let last_update: Option<DateTime<Utc>> = row.get("last_update");

        // Get total tokens separately with proper type handling
        let total_tokens = sqlx::query_scalar::<_, i64>(
            r"
            SELECT COALESCE(SUM(CAST(token_count AS BIGINT)), 0)::bigint
            FROM documents 
            WHERE doc_type::text = 'rust' 
            AND metadata->>'crate_name' IS NOT NULL
            ",
        )
        .fetch_one(pool)
        .await?;

        let average_docs_per_crate = if total_crates > 0 {
            #[allow(clippy::cast_precision_loss)] // Acceptable precision loss for statistics
            {
                total_docs as f64 / total_crates as f64
            }
        } else {
            0.0
        };

        Ok(crate::models::CrateStatistics {
            total_crates,
            active_crates,
            total_docs_managed: total_docs,
            total_tokens_managed: total_tokens,
            average_docs_per_crate,
            last_update,
        })
    }

    /// Check if crate exists by name
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn find_crate_by_name(
        pool: &PgPool,
        crate_name: &str,
    ) -> Result<Option<crate::models::CrateInfo>> {
        let row = sqlx::query(
            r"
            SELECT 
                COALESCE(metadata->>'crate_name', 'unknown') as name,
                COALESCE(metadata->>'crate_version', 'latest') as version,
                COUNT(*) as total_docs,
                COALESCE(SUM(CAST(token_count AS BIGINT)), 0)::bigint as total_tokens,
                MAX(created_at) as last_updated
            FROM documents 
            WHERE doc_type::text = 'rust' 
            AND metadata->>'crate_name' = $1
            GROUP BY metadata->>'crate_name', metadata->>'crate_version'
            LIMIT 1
            ",
        )
        .bind(crate_name)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            let name: String = row.get("name");
            let version: String = row.get("version");
            let total_docs: i64 = row.get("total_docs");
            let total_tokens: i64 = row.get("total_tokens");
            let last_updated: DateTime<Utc> = row.get("last_updated");

            Ok(Some(crate::models::CrateInfo {
                name,
                version,
                description: None,
                documentation_url: None,
                total_docs: i32::try_from(total_docs).unwrap_or(i32::MAX),
                total_tokens,
                last_updated,
            }))
        } else {
            Ok(None)
        }
    }
}
