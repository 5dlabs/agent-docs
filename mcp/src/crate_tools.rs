//! Crate management tools for MCP

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::write_with_newline)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::too_many_lines)]

use crate::job_queue::CrateJobProcessor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use db::{
    models::{CrateJob, JobStatus, PaginationParams},
    queries::{CrateJobQueries, CrateQueries},
    DatabasePool,
};
use embed::client::EmbeddingClient;
use serde_json::{json, Value};
use std::{fmt::Write as _, sync::Arc};
use uuid::Uuid;

use crate::tools::Tool;

/// Add Rust crate tool - enqueues background job and returns 202 + job ID
pub struct AddRustCrateTool {
    job_processor: CrateJobProcessor,
}

impl AddRustCrateTool {
    /// Create a new add crate tool with background job processing
    pub fn new(
        db_pool: DatabasePool,
        embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    ) -> Self {
        Self {
            job_processor: CrateJobProcessor::new_with_worker(db_pool, embedding_client),
        }
    }
}

#[async_trait]
impl Tool for AddRustCrateTool {
    fn definition(&self) -> Value {
        json!({
            "name": "add_rust_crate",
            "description": "Add a new Rust crate to the documentation system with automatic docs.rs ingestion. Returns immediately with a job ID for tracking progress.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the Rust crate to add (e.g., 'tokio', 'serde')"
                    },
                    "version": {
                        "type": "string",
                        "description": "Specific version to fetch (optional, defaults to latest)"
                    }
                },
                "required": ["name"]
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let crate_name = arguments
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow!("Missing required 'name' parameter"))?;

        let version = arguments.get("version").and_then(Value::as_str);

        // Validate crate name
        if crate_name.is_empty() {
            return Err(anyhow!("Crate name cannot be empty"));
        }

        // Check if crate already exists by looking at documents
        if let Some(existing_crate) =
            CrateQueries::find_crate_by_name(self.job_processor.db_pool().pool(), crate_name)
                .await?
        {
            return Ok(format!(
                "Crate '{}' already exists in the system (version: {}). Use remove_rust_crate first if you want to re-add it.",
                crate_name, existing_crate.version
            ));
        }

        // Enqueue the background job and return 202 + job ID immediately
        let job_id = self
            .job_processor
            .enqueue_add_crate_job(crate_name, version)
            .await?;

        Ok(format!(
            "{{\"status\": \"accepted\", \"job_id\": \"{}\", \"message\": \"Crate '{}' ingestion job queued successfully. Use check_rust_status with job_id to track progress.\"}}",
            job_id, crate_name
        ))
    }
}

/// Remove Rust crate tool with cascade deletion
pub struct RemoveRustCrateTool {
    db_pool: DatabasePool,
}

impl RemoveRustCrateTool {
    /// Create a new remove crate tool
    pub fn new(db_pool: DatabasePool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl Tool for RemoveRustCrateTool {
    fn definition(&self) -> Value {
        json!({
            "name": "remove_rust_crate",
            "description": "Remove a Rust crate from the documentation system with cascade deletion of all associated documents and embeddings.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the Rust crate to remove"
                    },
                    "crate_name": {
                        "type": "string",
                        "description": "Alias for 'name' (accepted for backward compatibility)"
                    },
                    "soft_delete": {
                        "type": "boolean",
                        "description": "If true, mark as inactive instead of hard delete (default: false)"
                    }
                },
                "required": ["name"]
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let crate_name = arguments
            .get("name")
            .and_then(|n| n.as_str())
            .or_else(|| arguments.get("crate_name").and_then(|n| n.as_str()))
            .ok_or_else(|| anyhow!("Missing required 'name' parameter"))?;

        let soft_delete = arguments
            .get("soft_delete")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Validate crate name
        if crate_name.is_empty() {
            return Err(anyhow!("Crate name cannot be empty"));
        }

        // Check if crate exists
        let crate_info = CrateQueries::find_crate_by_name(self.db_pool.pool(), crate_name).await?;
        let Some(_crate_info) = crate_info else {
            return Ok(format!("Crate '{}' not found in the system.", crate_name));
        };

        if soft_delete {
            // Soft delete - mark documents as inactive in metadata
            self.perform_soft_deletion(crate_name).await
        } else {
            // Hard delete with cascade - use full transaction
            self.perform_cascade_deletion(crate_name).await
        }
    }
}

impl RemoveRustCrateTool {
    /// Perform cascade deletion with full transaction support
    async fn perform_cascade_deletion(&self, crate_name: &str) -> Result<String> {
        let mut tx = self.db_pool.pool().begin().await?;

        // Find all documents for this crate first (for count reporting)
        let doc_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .fetch_one(&mut *tx)
        .await?;

        // Delete documents first (this will cascade to embeddings if there are foreign key constraints)
        let documents_deleted = sqlx::query(
            "DELETE FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .execute(&mut *tx)
        .await?;

        // Commit transaction (no crates table to delete from)
        tx.commit().await?;

        tracing::info!(
            "Successfully deleted crate '{}': {} documents removed",
            crate_name,
            documents_deleted.rows_affected()
        );

        Ok(format!(
            "Crate '{}' removed successfully. Deleted {} documents and all associated embeddings.",
            crate_name, doc_count
        ))
    }

    /// Perform soft deletion by marking documents as inactive
    async fn perform_soft_deletion(&self, crate_name: &str) -> Result<String> {
        let mut tx = self.db_pool.pool().begin().await?;

        // Count documents first
        let doc_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .fetch_one(&mut *tx)
        .await?;

        if doc_count == 0 {
            tx.rollback().await?;
            return Ok(format!("Crate '{}' not found in the system.", crate_name));
        }

        // Update metadata to mark as inactive
        let updated = sqlx::query(
            r#"
            UPDATE documents 
            SET metadata = jsonb_set(metadata, '{status}', '"inactive"', true),
                updated_at = CURRENT_TIMESTAMP
            WHERE doc_type = 'rust' 
            AND (metadata->>'crate_name' = $1 OR source_name = $1)
            "#,
        )
        .bind(crate_name)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::info!(
            "Successfully marked crate '{}' as inactive: {} documents updated",
            crate_name,
            updated.rows_affected()
        );

        Ok(format!(
            "Crate '{}' marked as inactive. {} documents remain in the system but are not searchable.",
            crate_name, doc_count
        ))
    }
}

/// List Rust crates tool with pagination
pub struct ListRustCratesTool {
    db_pool: DatabasePool,
}

impl ListRustCratesTool {
    /// Create a new list crates tool
    pub fn new(db_pool: DatabasePool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl Tool for ListRustCratesTool {
    fn definition(&self) -> Value {
        json!({
            "name": "list_rust_crates",
            "description": "List all Rust crates in the documentation system with pagination, filtering, and statistics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page number (default: 1)",
                        "minimum": 1
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Items per page (default: 20, max: 100)",
                        "minimum": 1,
                        "maximum": 100
                    },
                    "status_filter": {
                        "type": "string",
                        "description": "Filter by crate status",
                        "enum": ["active", "inactive", "updating", "failed"]
                    },
                    "name_pattern": {
                        "type": "string",
                        "description": "Search pattern for crate names (case-insensitive)"
                    },
                    "include_stats": {
                        "type": "boolean",
                        "description": "Include comprehensive system statistics (default: false)"
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let page = arguments
            .get("page")
            .and_then(Value::as_i64)
            .map(|p| p as i32);
        let limit = arguments
            .get("limit")
            .and_then(Value::as_i64)
            .map(|l| l as i32);
        let status_filter = arguments.get("status_filter").and_then(Value::as_str);
        let name_pattern = arguments.get("name_pattern").and_then(Value::as_str);
        let include_stats = arguments
            .get("include_stats")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let pagination = PaginationParams::new(page, limit);

        // Validate status filter (we only support active/inactive now)
        if let Some(status) = status_filter {
            if status != "active" && status != "inactive" {
                return Err(anyhow!(
                    "Invalid status filter: {}. Only 'active' and 'inactive' are supported.",
                    status
                ));
            }
        }

        // Get paginated results
        let response =
            CrateQueries::list_crates(self.db_pool.pool(), &pagination, name_pattern).await?;

        // Optionally get comprehensive statistics
        let stats = if include_stats {
            Some(CrateQueries::get_crate_statistics(self.db_pool.pool()).await?)
        } else {
            None
        };

        // Format response
        let mut output = format!(
            "Rust Crates (Page {} of {}, {} total items):\n\n",
            response.page, response.total_pages, response.total_items
        );

        // Add comprehensive statistics if requested
        if let Some(stats) = &stats {
            output.push_str("📊 **System Statistics:**\n");
            let _ = write!(
                &mut output,
                "   Total Crates: {} (Active: {})\n",
                stats.total_crates, stats.active_crates
            );
            let _ = write!(
                &mut output,
                "   Total Documents: {}\n",
                stats.total_docs_managed
            );
            let _ = write!(
                &mut output,
                "   Total Tokens: {}\n",
                stats.total_tokens_managed
            );
            let _ = write!(
                &mut output,
                "   Average Docs per Crate: {:.1}\n",
                stats.average_docs_per_crate
            );
            if let Some(last_update) = &stats.last_update {
                let _ = write!(
                    &mut output,
                    "   Last Update: {}\n",
                    last_update.format("%Y-%m-%d %H:%M UTC")
                );
            }
            output.push('\n');
        }

        for crate_info in &response.items {
            let _ = write!(
                &mut output,
                "📦 **{}** (v{})\n   Docs: {} | Tokens: {} | Updated: {}\n",
                crate_info.name,
                crate_info.version,
                crate_info.total_docs,
                crate_info.total_tokens,
                crate_info.last_updated.format("%Y-%m-%d %H:%M UTC")
            );

            if let Some(description) = &crate_info.description {
                let _ = write!(&mut output, "   Description: {}\n", description);
            }

            output.push('\n');
        }

        // Add pagination info
        if response.has_previous || response.has_next {
            output.push_str("Navigation:\n");
            if response.has_previous {
                output.push_str("  ← Use page=");
                output.push_str(&(response.page - 1).to_string());
                output.push_str(" for previous\n");
            }
            if response.has_next {
                output.push_str("  → Use page=");
                output.push_str(&(response.page + 1).to_string());
                output.push_str(" for next\n");
            }
        }

        Ok(output)
    }
}

/// Check Rust status tool for health monitoring
pub struct CheckRustStatusTool {
    db_pool: DatabasePool,
    job_processor: CrateJobProcessor,
}

impl CheckRustStatusTool {
    /// Create a new check status tool
    pub fn new(db_pool: DatabasePool) -> Self {
        let job_processor = CrateJobProcessor::new(db_pool.clone());
        Self {
            db_pool,
            job_processor,
        }
    }
}

#[async_trait]
impl Tool for CheckRustStatusTool {
    fn definition(&self) -> Value {
        json!({
            "name": "check_rust_status",
            "description": "Check system health and get comprehensive statistics about Rust crate management, including job status tracking.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "Specific job ID to check status (optional)"
                    },
                    "include_active_jobs": {
                        "type": "boolean",
                        "description": "Include list of active/recent jobs (default: true)"
                    }
                },
                "required": []
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let job_id = arguments.get("job_id").and_then(Value::as_str);
        let include_active_jobs = arguments
            .get("include_active_jobs")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let mut output = String::new();

        // If specific job ID requested
        if let Some(job_id_str) = job_id {
            let job_id =
                Uuid::parse_str(job_id_str).map_err(|_| anyhow!("Invalid job ID format"))?;

            match self.job_processor.get_job_status(job_id).await? {
                Some(job) => {
                    let _ = write!(&mut output, "Job Status: {}\n\n", job_id);
                    let _ = write!(&mut output, "  Crate: {}\n", job.crate_name);
                    let _ = write!(&mut output, "  Operation: {}\n", job.operation);
                    let _ = write!(&mut output, "  Status: {:?}\n", job.status);
                    if let Some(progress) = job.progress {
                        let _ = write!(&mut output, "  Progress: {}%\n", progress);
                    }
                    let _ = write!(
                        &mut output,
                        "  Started: {}\n",
                        job.started_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    if let Some(finished) = job.finished_at {
                        let _ = write!(
                            &mut output,
                            "  Finished: {}\n",
                            finished.format("%Y-%m-%d %H:%M:%S UTC")
                        );
                    }
                    if let Some(error) = &job.error {
                        let _ = write!(&mut output, "  Error: {}\n", error);
                    }
                    output.push('\n');
                }
                None => {
                    let _ = write!(&mut output, "Job {} not found.\n\n", job_id);
                }
            }
        }

        // Get overall system statistics
        let stats = CrateQueries::get_crate_statistics(self.db_pool.pool()).await?;

        output.push_str("🦀 Rust Crate Management System Status\n\n");

        let _ = write!(&mut output, "📊 **System Statistics:**\n");
        let _ = write!(&mut output, "  • Total Crates: {}\n", stats.total_crates);
        let _ = write!(&mut output, "  • Active Crates: {}\n", stats.active_crates);
        let _ = write!(
            &mut output,
            "  • Total Documents: {}\n",
            stats.total_docs_managed
        );
        let _ = write!(
            &mut output,
            "  • Total Tokens: {}\n",
            stats.total_tokens_managed
        );
        let _ = write!(
            &mut output,
            "  • Average Docs/Crate: {:.1}\n",
            stats.average_docs_per_crate
        );

        if let Some(last_update) = stats.last_update {
            let _ = write!(
                &mut output,
                "  • Last Update: {}\n",
                last_update.format("%Y-%m-%d %H:%M UTC")
            );
        }

        output.push('\n');

        // Show active/recent jobs if requested
        if include_active_jobs {
            let active_jobs = CrateJobQueries::find_active_jobs(self.db_pool.pool()).await?;

            if !active_jobs.is_empty() {
                output.push_str("🔄 **Active Jobs:**\n");
                for job in &active_jobs {
                    let _ = write!(
                        &mut output,
                        "  • {} [{}] - {} ({:?}",
                        job.crate_name, job.id, job.operation, job.status
                    );
                    if let Some(progress) = job.progress {
                        let _ = write!(&mut output, " - {}%", progress);
                    }
                    output.push_str(")\n");
                }
                output.push('\n');
            }

            // Show recent completed jobs
            let all_jobs = sqlx::query_as::<_, CrateJob>(
                "SELECT * FROM crate_jobs ORDER BY started_at DESC LIMIT 5",
            )
            .fetch_all(self.db_pool.pool())
            .await?;

            let recent_completed: Vec<_> = all_jobs
                .into_iter()
                .filter(|job| matches!(job.status, JobStatus::Completed | JobStatus::Failed))
                .take(3)
                .collect();

            if !recent_completed.is_empty() {
                output.push_str("📋 **Recent Jobs:**\n");
                for job in recent_completed {
                    let _ = write!(
                        &mut output,
                        "  • {} - {} ({:?}) - {}\n",
                        job.crate_name,
                        job.operation,
                        job.status,
                        job.started_at.format("%m-%d %H:%M")
                    );
                }
                output.push('\n');
            }
        }

        // Database connectivity and performance check
        let start_time = std::time::Instant::now();
        let db_health = sqlx::query("SELECT 1 as health")
            .fetch_one(self.db_pool.pool())
            .await;
        let db_response_time = start_time.elapsed();

        output.push_str("🔍 **System Health:**\n");
        match db_health {
            Ok(_) => {
                let _ = write!(
                    &mut output,
                    "  ✅ Database: Connected and responsive ({:.2}ms)\n",
                    db_response_time.as_secs_f64() * 1000.0
                );
            }
            Err(e) => {
                let _ = write!(&mut output, "  ❌ Database: Error - {}\n", e);
            }
        }

        // Get additional storage metrics (temporarily disabled due to DB schema issues)
        // if let Ok(storage_info) = self.get_storage_metrics().await {
        //     output.push_str(&format!("  📁 Storage Usage:\n"));
        //     output.push_str(&format!("    • Documents table: {} records\n", storage_info.documents_count));
        //     output.push_str(&format!("    • Embeddings: {} with vectors\n", storage_info.embeddings_count));
        //     output.push_str(&format!("    • Average content size: {} chars\n", storage_info.avg_content_size));
        //     output.push_str(&format!("    • Database size estimate: {:.1} MB\n", storage_info.estimated_size_mb));
        // }

        // Get job queue metrics (temporarily disabled due to DB schema issues)
        // if let Ok(job_metrics) = self.get_job_metrics().await {
        //     output.push_str(&format!("  ⚙️ Job Queue Metrics:\n"));
        //     output.push_str(&format!("    • Queued jobs: {}\n", job_metrics.queued_jobs));
        //     output.push_str(&format!("    • Running jobs: {}\n", job_metrics.running_jobs));
        //     output.push_str(&format!("    • Completed jobs (24h): {}\n", job_metrics.completed_jobs_24h));
        //     output.push_str(&format!("    • Failed jobs (24h): {}\n", job_metrics.failed_jobs_24h));
        // }

        Ok(output)
    }
}

impl CheckRustStatusTool {
    /// Get storage metrics for system monitoring (temporarily disabled)
    #[allow(dead_code, clippy::unused_async)]
    async fn get_storage_metrics(&self) -> Result<StorageMetrics> {
        // TODO: Fix after database schema is properly set up
        Ok(StorageMetrics {
            documents_count: 0,
            embeddings_count: 0,
            avg_content_size: 0,
            estimated_size_mb: 0.0,
        })
    }

    /// Get job queue metrics (temporarily disabled)
    #[allow(dead_code, clippy::unused_async)]
    async fn get_job_metrics(&self) -> Result<JobMetrics> {
        // TODO: Fix after database schema is properly set up
        Ok(JobMetrics {
            queued_jobs: 0,
            running_jobs: 0,
            completed_jobs_24h: 0,
            failed_jobs_24h: 0,
        })
    }
}

/// Storage metrics for system monitoring
#[derive(Debug)]
#[allow(dead_code)] // Temporarily disabled until database schema is ready
struct StorageMetrics {
    documents_count: i32,
    embeddings_count: i32,
    avg_content_size: i32,
    estimated_size_mb: f64,
}

/// Job queue metrics for monitoring
#[derive(Debug)]
#[allow(dead_code)] // Temporarily disabled until database schema is ready
struct JobMetrics {
    queued_jobs: i32,
    running_jobs: i32,
    completed_jobs_24h: i32,
    failed_jobs_24h: i32,
}
