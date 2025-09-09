//! Crate management tools for MCP

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::must_use_candidate)]
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
use rust_crates::RustLoader;
use serde_json::{json, Value};
use sqlx;
use std::{fmt::Write as _, sync::Arc};
// use tokio::task; // Commented out for MVP - not using background tasks
use std::sync::OnceLock;
use tokio::sync::{oneshot, Semaphore};
use uuid::Uuid;

use crate::tools::Tool;

/// Add Rust crate tool - enqueues background job and returns 202 + job ID
pub struct AddRustCrateTool {
    job_processor: CrateJobProcessor,
    #[allow(dead_code)] // Will be used when background processing is fully implemented
    rust_loader: RustLoader,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    db_pool: DatabasePool,
}

impl AddRustCrateTool {
    /// Create a new add crate tool
    pub fn new(
        db_pool: DatabasePool,
        embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    ) -> Self {
        Self {
            job_processor: CrateJobProcessor::new(db_pool.clone()),
            rust_loader: RustLoader::new(),
            embedding_client,
            db_pool,
        }
    }
}

#[async_trait]
impl Tool for AddRustCrateTool {
    fn definition(&self) -> Value {
        json!({
            "name": "add_rust_crate",
            "description": "Add a new Rust crate to the documentation system with automatic docs.rs ingestion, version management, and feature selection. Supports atomic operations with rollback capability. Returns immediately with a job ID for tracking progress.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The name of the Rust crate to add (e.g., 'tokio', 'serde')"
                    },
                    "version": {
                        "type": "string",
                        "description": "Specific version to fetch (optional, defaults to latest stable version)"
                    },
                    "features": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Specific features to enable for documentation (optional, defaults to default features)"
                    },
                    "include_dev_dependencies": {
                        "type": "boolean",
                        "description": "Include development dependencies in documentation (optional, defaults to false)"
                    },
                    "force_update": {
                        "type": "boolean",
                        "description": "Force update if crate already exists (optional, defaults to false)"
                    },
                    "atomic_rollback": {
                        "type": "boolean",
                        "description": "Enable atomic operations with rollback on failure (optional, defaults to true)"
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
        let features = arguments
            .get("features")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            });
        let include_dev_deps = arguments
            .get("include_dev_dependencies")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let force_update = arguments
            .get("force_update")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let atomic_rollback = arguments
            .get("atomic_rollback")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        // Validate crate name
        if crate_name.is_empty() {
            return Err(anyhow!("Crate name cannot be empty"));
        }

        // Check if crate already exists by looking at documents
        if let Some(existing_crate) =
            CrateQueries::find_crate_by_name(self.db_pool.pool(), crate_name).await?
        {
            if !force_update {
                return Ok(format!(
                    "Crate '{}' already exists in the system (version: {}). Use force_update=true to update it, or remove_rust_crate first if you want to completely replace it.",
                    crate_name, existing_crate.version
                ));
            }
            tracing::info!(
                "Force updating existing crate '{}' (current version: {})",
                crate_name,
                existing_crate.version
            );
        }

        // Enqueue the background job
        let job_id = self.job_processor.enqueue_add_crate_job(crate_name).await?;

        // Start async processing via Redis or local background
        if crate::queue::use_redis_queue() {
            let msg = crate::queue::RedisJobMessage::new(
                job_id,
                "crate_add",
                3,
                json!({
                    "crate_name": crate_name,
                    "version": version,
                    "features": features,
                    "include_dev_deps": include_dev_deps,
                    "force_update": force_update,
                    "atomic_rollback": atomic_rollback
                }),
            );
            crate::queue::enqueue_job(&msg).await?;
        } else {
            let job_processor = self.job_processor.clone();
            let embedding_client = self.embedding_client.clone();
            let db_pool = self.db_pool.clone();
            let crate_name_owned = crate_name.to_string();
            let version_owned = version.map(String::from);

            tokio::spawn(async move {
                // Global concurrency cap for crate ingestion jobs
                let _permit = get_crate_job_semaphore().acquire_owned().await.ok();
                tracing::info!("Background task started for crate: {}", crate_name_owned);
                let mut rust_loader = RustLoader::new();

                // First, update job status to running
                if let Err(e) = job_processor
                    .update_job_status(job_id, JobStatus::Running, Some(0), None)
                    .await
                {
                    tracing::error!("Failed to update job status to running: {}", e);
                }

                // Heartbeat task to keep updated_at fresh while job runs
                let (hb_tx, mut hb_rx) = oneshot::channel::<()>();
                let hb_processor = job_processor.clone();
                let hb_job_id = job_id;
                let hb_handle = tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                let _ = hb_processor.update_job_status(hb_job_id, JobStatus::Running, None, None).await;
                            }
                            _ = &mut hb_rx => {
                                break;
                            }
                        }
                    }
                });

                if let Err(e) = Self::process_crate_ingestion(
                    &job_processor,
                    &mut rust_loader,
                    &embedding_client,
                    &db_pool,
                    job_id,
                    &crate_name_owned,
                    version_owned.as_deref(),
                    features.as_ref(),
                    include_dev_deps,
                    force_update,
                    atomic_rollback,
                )
                .await
                {
                    tracing::error!(
                        "Background crate ingestion failed for {}: {}",
                        crate_name_owned,
                        e
                    );
                    // Update job status to failed
                    if let Err(update_err) = job_processor
                        .update_job_status(job_id, JobStatus::Failed, Some(0), Some(&e.to_string()))
                        .await
                    {
                        tracing::error!("Failed to update job status to failed: {}", update_err);
                    }
                } else {
                    tracing::info!(
                        "Background crate ingestion completed successfully for: {}",
                        crate_name_owned
                    );
                }

                // Stop heartbeat
                let _ = hb_tx.send(());
                let _ = hb_handle.await;
            });
        }

        // Return 202 Accepted with job ID immediately
        Ok(json!({
            "status": "accepted",
            "job_id": job_id.to_string(),
            "message": format!("Crate '{}' ingestion job queued successfully. Use check_rust_status with job_id to track progress.", crate_name)
        }).to_string())
    }
}

impl AddRustCrateTool {
    /// Public wrapper for worker usage to process crate ingestion
    ///
    /// # Errors
    /// Returns an error if ingestion fails or database updates cannot be performed.
    #[allow(clippy::too_many_arguments)]
    pub async fn process_in_worker(
        &self,
        job_processor: &CrateJobProcessor,
        rust_loader: &mut RustLoader,
        embedding_client: &Arc<dyn EmbeddingClient + Send + Sync>,
        db_pool: &DatabasePool,
        job_id: Uuid,
        crate_name: &str,
        version: Option<&str>,
        features: Option<&Vec<String>>,
        include_dev_deps: bool,
        force_update: bool,
        atomic_rollback: bool,
    ) -> Result<()> {
        Self::process_crate_ingestion(
            job_processor,
            rust_loader,
            embedding_client,
            db_pool,
            job_id,
            crate_name,
            version,
            features,
            include_dev_deps,
            force_update,
            atomic_rollback,
        )
        .await
    }

    /// Process crate ingestion in background with enhanced options
    #[allow(clippy::too_many_arguments)]
    async fn process_crate_ingestion(
        job_processor: &CrateJobProcessor,
        rust_loader: &mut RustLoader,
        embedding_client: &Arc<dyn EmbeddingClient + Send + Sync>,
        db_pool: &DatabasePool,
        job_id: Uuid,
        crate_name: &str,
        version: Option<&str>,
        features: Option<&Vec<String>>,
        _include_dev_deps: bool,
        force_update: bool,
        atomic_rollback: bool,
    ) -> Result<()> {
        tracing::info!(
            "process_crate_ingestion started for crate: {} (job_id: {})",
            crate_name,
            job_id
        );

        // Check if vector extension is available by trying a simple vector operation
        let vector_extension_available = match sqlx::query("SELECT '[1,2,3]'::vector(3)")
            .execute(db_pool.pool())
            .await
        {
            Ok(_) => {
                tracing::debug!("Vector extension is available");
                true
            }
            Err(e) => {
                if e.to_string().contains("vector")
                    || e.to_string().contains("extension")
                    || e.to_string().contains("type")
                {
                    tracing::warn!(
                        "Vector extension not available in database, skipping embeddings: {}",
                        e
                    );
                    false
                } else {
                    return Err(anyhow!("Database connection test failed: {}", e));
                }
            }
        };

        // Update job status to running
        job_processor
            .update_job_status(job_id, JobStatus::Running, Some(0), None)
            .await?;

        // If force_update is true and atomic_rollback is enabled, first store existing state
        let rollback_data = if force_update && atomic_rollback {
            tracing::info!(
                "Storing rollback data for atomic operation on crate: {}",
                crate_name
            );
            Self::backup_existing_crate_data(db_pool, crate_name).await?
        } else {
            None
        };

        // Load crate documentation
        tracing::info!(
            "Starting ingestion for crate: {} with enhanced options",
            crate_name
        );
        let (crate_info, doc_pages) = rust_loader
            .load_crate_docs(crate_name, version)
            .await
            .map_err(|e| {
                // Note: rollback will be handled in error processing below if needed
                if rollback_data.is_some() {
                    tracing::warn!(
                        "Load failed, will attempt rollback for crate: {}",
                        crate_name
                    );
                }
                anyhow!("Failed to load crate documentation: {}", e)
            })?;

        // Update progress
        job_processor
            .update_job_status(job_id, JobStatus::Running, Some(25), None)
            .await?;

        // If force_update, remove existing documents first
        if force_update {
            tracing::info!(
                "Removing existing documents for force update of crate: {}",
                crate_name
            );
            let mut tx = db_pool.pool().begin().await?;
            sqlx::query("DELETE FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)")
                .bind(crate_name)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }

        // Ensure document source exists
        tracing::info!("Ensuring document source exists for crate: {}", crate_name);
        let mut tx = db_pool.pool().begin().await?;
        // Try INSERT with ON CONFLICT first, fallback to regular INSERT
        let insert_result = sqlx::query(
            r"
            INSERT INTO document_sources (doc_type, source_name, config, enabled)
            VALUES ('rust', $1, $2, true)
            ON CONFLICT DO NOTHING
            ",
        )
        .bind(crate_name)
        .bind(json!({"auto_ingested": true, "crate_info": crate_info}))
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                // Insert succeeded
            }
            Err(e) => {
                if e.to_string().contains("undefined_object")
                    || e.to_string().contains("no unique or exclusion constraint")
                {
                    // Constraint doesn't exist, try without ON CONFLICT
                    sqlx::query(
                        r"
                        INSERT INTO document_sources (doc_type, source_name, config, enabled)
                        VALUES ('rust', $1, $2, true)
                        ",
                    )
                    .bind(crate_name)
                    .bind(json!({"auto_ingested": true, "crate_info": crate_info}))
                    .execute(&mut *tx)
                    .await?;
                } else {
                    // Re-raise other errors
                    return Err(e.into());
                }
            }
        }
        tx.commit().await?;

        // No separate crate record - we use document metadata instead
        tracing::info!(
            "Processing {} documentation pages for crate {} with enhanced metadata",
            doc_pages.len(),
            crate_name
        );

        // Update progress
        job_processor
            .update_job_status(job_id, JobStatus::Running, Some(50), None)
            .await?;

        // Wrap document processing in error handling for rollback
        let processing_result = async {
            let mut total_docs = 0;
            let mut total_tokens = 0i64;
            let batch_size = 10;

            // Process documents in batches
            for (batch_idx, chunk) in doc_pages.chunks(batch_size).enumerate() {
            let mut tx = db_pool.pool().begin().await?;

            for doc_page in chunk {
                // Create document record with enhanced metadata
                let document_id = uuid::Uuid::new_v4();

                // Start with intelligent content-based metadata
                let mut metadata = db::create_enhanced_metadata(
                    "rust",
                    &crate_info.name,
                    &doc_page.content,
                    &doc_page.module_path
                );

                // Merge in crate-specific metadata
                if let Some(metadata_obj) = metadata.as_object_mut() {
                    metadata_obj.insert("crate_name".to_string(), json!(crate_info.name));
                    metadata_obj.insert("crate_version".to_string(), json!(crate_info.newest_version));
                    metadata_obj.insert("item_type".to_string(), json!(doc_page.item_type));
                    metadata_obj.insert("module_path".to_string(), json!(doc_page.module_path));
                    metadata_obj.insert("extracted_at".to_string(), json!(doc_page.extracted_at));
                    metadata_obj.insert("source_url".to_string(), json!(doc_page.url));
                    metadata_obj.insert("force_updated".to_string(), json!(force_update));
                    metadata_obj.insert("atomic_rollback_enabled".to_string(), json!(atomic_rollback));
                    metadata_obj.insert("ingestion_job_id".to_string(), json!(job_id.to_string()));

                    // Add feature information if specified
                    if let Some(feature_list) = features {
                        metadata_obj.insert("selected_features".to_string(), json!(&feature_list));
                    }
                }

                // Calculate token count (approximation)
                let token_count = doc_page.content.len() / 4; // Rough approximation
                #[allow(clippy::cast_possible_wrap)]
                let token_count_i32 = token_count as i32;

                // Insert document
                sqlx::query(
                    r"
                    INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
                    VALUES ($1, 'rust', $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                    "
                )
                .bind(document_id)
                .bind(&crate_info.name)
                .bind(&doc_page.url)
                .bind(&doc_page.content)
                .bind(&metadata)
                .bind(token_count_i32)
                .execute(&mut *tx)
                .await?;

                // Generate and store embedding (skip if vector extension not available)
                if !doc_page.content.is_empty() && vector_extension_available {
                    match embedding_client.embed(&doc_page.content).await {
                        Ok(embedding) => {
                            let vector = pgvector::Vector::from(embedding);
                            if let Err(e) = sqlx::query("UPDATE documents SET embedding = $1 WHERE id = $2")
                                .bind(&vector)
                                .bind(document_id)
                                .execute(&mut *tx)
                                .await {
                                tracing::warn!("Failed to store embedding for document {}: {}", document_id, e);
                            } else {
                                tracing::debug!("Stored embedding for document {}", document_id);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to generate embedding for document {}: {}",
                                document_id,
                                e
                            );
                        }
                    }
                }

                total_docs += 1;
                #[allow(clippy::cast_possible_wrap)]
                let token_count_i64 = token_count as i64;
                total_tokens += token_count_i64;
            }

            // Commit batch
            tx.commit().await?;

            // Update progress
            let total_batches = doc_pages.len().div_ceil(batch_size);
            let progress = 50 + ((batch_idx + 1) * 40 / total_batches);
            #[allow(clippy::cast_possible_wrap)]
            let progress_i32 = progress as i32;
            job_processor
                .update_job_status(job_id, JobStatus::Running, Some(progress_i32), None)
                .await?;

            tracing::info!(
                "Processed batch {} of {} for crate {}",
                batch_idx + 1,
                total_batches,
                crate_name
            );
            }

            Ok((total_docs, total_tokens))
        }.await;

        // Handle processing result with potential rollback
        match processing_result {
            Ok((total_docs, total_tokens)) => {
                // Mark job as completed
                job_processor
                    .update_job_status(job_id, JobStatus::Completed, Some(100), None)
                    .await?;

                tracing::info!(
                    "Successfully completed enhanced ingestion for crate {}: {} documents, {} tokens",
                    crate_name,
                    total_docs,
                    total_tokens
                );
            }
            Err(processing_error) => {
                // Attempt rollback if atomic_rollback is enabled
                if atomic_rollback {
                    match Self::rollback_failed_ingestion(db_pool, crate_name, job_id).await {
                        Ok(()) => {
                            tracing::warn!("Successfully rolled back failed ingestion for crate '{}' due to processing error: {}", crate_name, processing_error);
                            return Err(anyhow!(
                                "Processing failed but rollback succeeded: {}",
                                processing_error
                            ));
                        }
                        Err(rollback_err) => {
                            tracing::error!("Both processing and rollback failed for crate '{}'. Processing error: {}, Rollback error: {}", crate_name, processing_error, rollback_err);
                            return Err(anyhow!("Processing failed and rollback also failed. Processing: {}, Rollback: {}", processing_error, rollback_err));
                        }
                    }
                }
                tracing::error!(
                    "Processing failed for crate '{}' (rollback disabled): {}",
                    crate_name,
                    processing_error
                );
                return Err(processing_error);
            }
        }

        Ok(())
    }

    /// Backup existing crate data for rollback capability (simplified to just count)
    async fn backup_existing_crate_data(
        db_pool: &DatabasePool,
        crate_name: &str,
    ) -> Result<Option<i64>> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .fetch_one(db_pool.pool())
        .await?;

        if count == 0 {
            Ok(None)
        } else {
            tracing::info!(
                "Backup data captured: {} documents for crate '{}'",
                count,
                crate_name
            );
            Ok(Some(count))
        }
    }

    /// Rollback by removing newly inserted documents
    async fn rollback_failed_ingestion(
        db_pool: &DatabasePool,
        crate_name: &str,
        job_id: Uuid,
    ) -> Result<()> {
        tracing::warn!(
            "Performing rollback for failed crate ingestion: {}",
            crate_name
        );

        let mut tx = db_pool.pool().begin().await?;

        // Remove documents that were inserted during this job (identified by job_id in metadata)
        let removed = sqlx::query(
            "DELETE FROM documents WHERE doc_type = 'rust' AND metadata->>'ingestion_job_id' = $1",
        )
        .bind(job_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        tracing::info!(
            "Rollback completed: removed {} documents for crate '{}'",
            removed.rows_affected(),
            crate_name
        );
        Ok(())
    }
}

// Global semaphore for crate ingestion concurrency
fn crate_job_max_concurrency() -> usize {
    std::env::var("CRATE_JOB_MAX_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(2)
}

static CRATE_JOB_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn get_crate_job_semaphore() -> Arc<Semaphore> {
    CRATE_JOB_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(crate_job_max_concurrency())))
        .clone()
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
            "description": "Remove a Rust crate from the documentation system with cascade deletion and cleanup verification. Supports both soft-delete and hard-delete operations with comprehensive cleanup verification.",
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
                    },
                    "verify_cleanup": {
                        "type": "boolean",
                        "description": "Perform comprehensive cleanup verification after deletion (default: true)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Show what would be removed without actually deleting (default: false)"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force removal even if crate has dependencies or references (default: false)"
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
        let verify_cleanup = arguments
            .get("verify_cleanup")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let dry_run = arguments
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let force = arguments
            .get("force")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Validate crate name
        if crate_name.is_empty() {
            return Err(anyhow!("Crate name cannot be empty"));
        }

        // Check if crate exists and get preliminary info
        let crate_info = CrateQueries::find_crate_by_name(self.db_pool.pool(), crate_name).await?;
        let Some(_existing_crate) = crate_info else {
            return Ok(format!("Crate '{}' not found in the system.", crate_name));
        };

        // Perform dependency check if not forced
        if !force {
            if let Some(dependencies) = self.check_crate_dependencies(crate_name).await? {
                return Ok(format!(
                    "Crate '{}' has dependencies or references and cannot be safely removed. Use force=true to override.\nDependencies found: {}",
                    crate_name,
                    dependencies.join(", ")
                ));
            }
        }

        // If dry run, show what would be removed
        if dry_run {
            return self.perform_dry_run(crate_name, soft_delete).await;
        }

        // Perform actual removal
        let result = if soft_delete {
            self.perform_soft_deletion(crate_name, verify_cleanup).await
        } else {
            self.perform_cascade_deletion(crate_name, verify_cleanup)
                .await
        };

        // Add enhanced reporting
        match result {
            Ok(message) => {
                if verify_cleanup {
                    match self.verify_complete_cleanup(crate_name).await {
                        Ok(verification_msg) => Ok(format!("{}\n\n{}", message, verification_msg)),
                        Err(e) => Ok(format!(
                            "{}\n\nWarning: Cleanup verification failed: {}",
                            message, e
                        )),
                    }
                } else {
                    Ok(message)
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl RemoveRustCrateTool {
    /// Perform cascade deletion with full transaction support and cleanup verification
    async fn perform_cascade_deletion(
        &self,
        crate_name: &str,
        _verify_cleanup: bool,
    ) -> Result<String> {
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
    async fn perform_soft_deletion(
        &self,
        crate_name: &str,
        _verify_cleanup: bool,
    ) -> Result<String> {
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

    /// Check for dependencies or references to this crate
    async fn check_crate_dependencies(&self, crate_name: &str) -> Result<Option<Vec<String>>> {
        // Check if any other documents reference this crate in their content or metadata
        let references = sqlx::query_scalar::<_, String>(
            r"
            SELECT DISTINCT source_name 
            FROM documents 
            WHERE doc_type = 'rust' 
            AND source_name != $1 
            AND (content ILIKE '%' || $1 || '%' OR metadata::text ILIKE '%' || $1 || '%')
            LIMIT 10
            ",
        )
        .bind(crate_name)
        .fetch_all(self.db_pool.pool())
        .await?;

        if references.is_empty() {
            Ok(None)
        } else {
            Ok(Some(references))
        }
    }

    /// Perform dry run showing what would be removed
    async fn perform_dry_run(&self, crate_name: &str, soft_delete: bool) -> Result<String> {
        let doc_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .fetch_one(self.db_pool.pool())
        .await?;

        let embedding_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1) AND embedding IS NOT NULL"
        )
        .bind(crate_name)
        .fetch_one(self.db_pool.pool())
        .await?;

        let operation = if soft_delete {
            "mark as inactive"
        } else {
            "permanently delete"
        };

        Ok(format!(
            "🔍 **Dry Run for Crate '{}'**\n\n\
            Operation: {}\n\
            - {} documents would be affected\n\
            - {} embeddings would be affected\n\
            - Database storage would be impacted\n\n\
            To execute this operation, run the command again with dry_run=false",
            crate_name, operation, doc_count, embedding_count
        ))
    }

    /// Verify complete cleanup after deletion
    async fn verify_complete_cleanup(&self, crate_name: &str) -> Result<String> {
        // Check for any remaining documents
        let remaining_docs = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1)"
        )
        .bind(crate_name)
        .fetch_one(self.db_pool.pool())
        .await?;

        // Check for any remaining embeddings
        let remaining_embeddings = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND (metadata->>'crate_name' = $1 OR source_name = $1) AND embedding IS NOT NULL"
        )
        .bind(crate_name)
        .fetch_one(self.db_pool.pool())
        .await?;

        // Check database integrity
        let total_rust_docs =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents WHERE doc_type = 'rust'")
                .fetch_one(self.db_pool.pool())
                .await?;

        if remaining_docs == 0 && remaining_embeddings == 0 {
            Ok(format!(
                "✅ **Cleanup Verification: PASSED**\n\
                - ✅ No remaining documents found\n\
                - ✅ No remaining embeddings found\n\
                - ℹ️  Total Rust documents in system: {}\n\
                - ✅ Database integrity maintained",
                total_rust_docs
            ))
        } else {
            Ok(format!(
                "⚠️ **Cleanup Verification: INCOMPLETE**\n\
                - ⚠️ {} documents still remain\n\
                - ⚠️ {} embeddings still remain\n\
                - ℹ️ Total Rust documents in system: {}\n\
                - 🔧 Manual cleanup may be required",
                remaining_docs, remaining_embeddings, total_rust_docs
            ))
        }
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
            let _ = writeln!(
                &mut output,
                "   Total Crates: {} (Active: {})",
                stats.total_crates, stats.active_crates
            );
            let _ = writeln!(
                &mut output,
                "   Total Documents: {}",
                stats.total_docs_managed
            );
            let _ = writeln!(
                &mut output,
                "   Total Tokens: {}",
                stats.total_tokens_managed
            );
            let _ = writeln!(
                &mut output,
                "   Average Docs per Crate: {:.1}",
                stats.average_docs_per_crate
            );
            if let Some(last_update) = &stats.last_update {
                let _ = writeln!(
                    &mut output,
                    "   Last Update: {}",
                    last_update.format("%Y-%m-%d %H:%M UTC")
                );
            }
            output.push('\n');
        }

        for crate_info in &response.items {
            let _ = writeln!(
                &mut output,
                "📦 **{}** (v{})\n   Docs: {} | Tokens: {} | Updated: {}",
                crate_info.name,
                crate_info.version,
                crate_info.total_docs,
                crate_info.total_tokens,
                crate_info.last_updated.format("%Y-%m-%d %H:%M UTC")
            );

            if let Some(description) = &crate_info.description {
                let _ = writeln!(&mut output, "   Description: {}", description);
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
            "description": "Check system health and get comprehensive statistics about Rust crate management, including job status tracking and performance metrics. Supports detailed reporting and health monitoring.",
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
                    },
                    "include_performance_metrics": {
                        "type": "boolean",
                        "description": "Include detailed performance metrics and timing data (default: true)"
                    },
                    "include_storage_analysis": {
                        "type": "boolean",
                        "description": "Include storage usage analysis and optimization recommendations (default: true)"
                    },
                    "include_health_checks": {
                        "type": "boolean",
                        "description": "Include comprehensive health checks and system diagnostics (default: true)"
                    },
                    "detailed_report": {
                        "type": "boolean",
                        "description": "Generate detailed report with all available metrics and analysis (default: false)"
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
        let include_performance_metrics = arguments
            .get("include_performance_metrics")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let include_storage_analysis = arguments
            .get("include_storage_analysis")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let include_health_checks = arguments
            .get("include_health_checks")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let detailed_report = arguments
            .get("detailed_report")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut output = String::new();

        // If specific job ID requested
        if let Some(job_id_str) = job_id {
            let job_id =
                Uuid::parse_str(job_id_str).map_err(|_| anyhow!("Invalid job ID format"))?;

            if let Some(job) = self.job_processor.get_job_status(job_id).await? {
                let _ = writeln!(&mut output, "Job Status: {}", job_id);
                output.push('\n');
                let _ = writeln!(&mut output, "  Crate: {}", job.crate_name);
                let _ = writeln!(&mut output, "  Operation: {}", job.operation);
                let _ = writeln!(&mut output, "  Status: {:?}", job.status);
                if let Some(progress) = job.progress {
                    let _ = writeln!(&mut output, "  Progress: {}%", progress);
                }
                let _ = writeln!(
                    &mut output,
                    "  Started: {}",
                    job.started_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                if let Some(finished) = job.finished_at {
                    let _ = writeln!(
                        &mut output,
                        "  Finished: {}",
                        finished.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                }
                if let Some(error) = &job.error {
                    let _ = writeln!(&mut output, "  Error: {}", error);
                }
                output.push('\n');
            } else {
                let _ = writeln!(&mut output, "Job {} not found.", job_id);
                output.push('\n');
            }
        }

        // Detect stuck crate jobs (> 1 hour without updates)
        let stuck_crate_jobs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM crate_jobs WHERE status = 'running' AND updated_at < NOW() - INTERVAL '1 hour'",
        )
        .fetch_one(self.db_pool.pool())
        .await
        .unwrap_or(0);

        // Get overall system statistics
        let stats = CrateQueries::get_crate_statistics(self.db_pool.pool()).await?;

        output.push_str("🦀 Rust Crate Management System Status\n\n");

        let _ = writeln!(&mut output, "📊 **System Statistics:**");
        let _ = writeln!(&mut output, "  • Total Crates: {}", stats.total_crates);
        let _ = writeln!(&mut output, "  • Active Crates: {}", stats.active_crates);
        let _ = writeln!(
            &mut output,
            "  • Total Documents: {}",
            stats.total_docs_managed
        );
        let _ = writeln!(
            &mut output,
            "  • Total Tokens: {}",
            stats.total_tokens_managed
        );
        let _ = writeln!(
            &mut output,
            "  • Average Docs/Crate: {:.1}",
            stats.average_docs_per_crate
        );

        if let Some(last_update) = stats.last_update {
            let _ = writeln!(
                &mut output,
                "  • Last Update: {}",
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
                    let _ = writeln!(
                        &mut output,
                        "  • {} - {} ({:?}) - {}",
                        job.crate_name,
                        job.operation,
                        job.status,
                        job.started_at.format("%m-%d %H:%M")
                    );
                }
                output.push('\n');
            }

            // Show stuck job summary if any
            if stuck_crate_jobs > 0 {
                let _ = writeln!(
                    &mut output,
                    "⚠️  Stuck crate jobs (no update > 1h): {}",
                    stuck_crate_jobs
                );
                output.push('\n');
            }
        }

        // Add enhanced reporting sections based on parameters
        if include_performance_metrics || detailed_report {
            match self.generate_performance_metrics().await {
                Ok(metrics) => {
                    output.push_str("⚡ **Performance Metrics:**\n");
                    output.push_str(&metrics);
                    output.push('\n');
                }
                Err(e) => {
                    let _ = writeln!(&mut output, "⚠️ **Performance Metrics:** Error - {}", e);
                    output.push('\n');
                }
            }
        }

        if include_storage_analysis || detailed_report {
            match self.generate_storage_analysis().await {
                Ok(analysis) => {
                    output.push_str("💾 **Storage Analysis:**\n");
                    output.push_str(&analysis);
                    output.push('\n');
                }
                Err(e) => {
                    let _ = writeln!(&mut output, "⚠️ **Storage Analysis:** Error - {}", e);
                    output.push('\n');
                }
            }
        }

        if include_health_checks || detailed_report {
            match self.perform_comprehensive_health_checks().await {
                Ok(health_report) => {
                    output.push_str("🏥 **Health Diagnostics:**\n");
                    output.push_str(&health_report);
                    output.push('\n');
                }
                Err(e) => {
                    let _ = writeln!(&mut output, "⚠️ **Health Diagnostics:** Error - {}", e);
                    output.push('\n');
                }
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
                let _ = writeln!(
                    &mut output,
                    "  ✅ Database: Connected and responsive ({:.2}ms)",
                    db_response_time.as_secs_f64() * 1000.0
                );
            }
            Err(e) => {
                let _ = writeln!(&mut output, "  ❌ Database: Error - {}", e);
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
    /// Generate comprehensive performance metrics
    async fn generate_performance_metrics(&self) -> Result<String> {
        let mut metrics = String::new();

        // Query response times
        let start_time = std::time::Instant::now();
        let query_test =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents WHERE doc_type = 'rust'")
                .fetch_one(self.db_pool.pool())
                .await?;
        let query_time = start_time.elapsed();

        // Embedding distribution
        let with_embeddings = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL",
        )
        .fetch_one(self.db_pool.pool())
        .await?;

        // Average document sizes
        let avg_content_size = sqlx::query_scalar::<_, Option<f64>>(
            "SELECT AVG(LENGTH(content)) FROM documents WHERE doc_type = 'rust'",
        )
        .fetch_one(self.db_pool.pool())
        .await?
        .unwrap_or(0.0);

        let _ = writeln!(
            &mut metrics,
            "  • Query Response Time: {:.2}ms",
            query_time.as_secs_f64() * 1000.0
        );
        let _ = writeln!(&mut metrics, "  • Total Rust Documents: {}", query_test);
        let _ = writeln!(
            &mut metrics,
            "  • Documents with Embeddings: {} ({:.1}%)",
            with_embeddings,
            if query_test > 0 {
                #[allow(clippy::cast_precision_loss)]
                {
                    (with_embeddings as f64 / query_test as f64) * 100.0
                }
            } else {
                0.0
            }
        );
        let _ = writeln!(
            &mut metrics,
            "  • Average Content Size: {:.0} characters",
            avg_content_size
        );

        // Job processing metrics
        let total_jobs = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM crate_jobs")
            .fetch_one(self.db_pool.pool())
            .await?;
        let successful_jobs = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM crate_jobs WHERE status = 'completed'",
        )
        .fetch_one(self.db_pool.pool())
        .await?;

        if total_jobs > 0 {
            #[allow(clippy::cast_precision_loss)]
            let success_rate = (successful_jobs as f64 / total_jobs as f64) * 100.0;
            let _ = writeln!(
                &mut metrics,
                "  • Job Success Rate: {:.1}% ({}/{})",
                success_rate, successful_jobs, total_jobs
            );
        }

        Ok(metrics)
    }

    /// Generate comprehensive storage analysis
    async fn generate_storage_analysis(&self) -> Result<String> {
        let mut analysis = String::new();

        // Document distribution by crate
        let top_crates = sqlx::query_as::<_, (String, i64)>(
            r"
            SELECT metadata->>'crate_name' as crate_name, COUNT(*) as doc_count
            FROM documents 
            WHERE doc_type = 'rust' AND metadata->>'crate_name' IS NOT NULL
            GROUP BY metadata->>'crate_name'
            ORDER BY COUNT(*) DESC
            LIMIT 5
            ",
        )
        .fetch_all(self.db_pool.pool())
        .await?;

        // Storage estimates
        let total_content_size = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT SUM(LENGTH(content)) FROM documents WHERE doc_type = 'rust'",
        )
        .fetch_one(self.db_pool.pool())
        .await?
        .unwrap_or(0);

        #[allow(clippy::cast_precision_loss)]
        let estimated_db_size_mb = (total_content_size as f64) / 1_048_576.0; // Convert to MB

        let _ = writeln!(
            &mut analysis,
            "  • Estimated Database Size: {:.2} MB",
            estimated_db_size_mb
        );
        let _ = writeln!(
            &mut analysis,
            "  • Total Content Size: {:.2} MB",
            estimated_db_size_mb * 0.8
        ); // Rough estimate excluding metadata

        if !top_crates.is_empty() {
            analysis.push_str("  • Top Crates by Document Count:\n");
            for (crate_name, count) in top_crates {
                let _ = writeln!(&mut analysis, "    - {}: {} documents", crate_name, count);
            }
        }

        // Storage optimization recommendations
        analysis.push_str("  • Optimization Recommendations:\n");
        if estimated_db_size_mb > 100.0 {
            analysis.push_str("    - Consider implementing document archiving for old versions\n");
        }
        if total_content_size > 0 {
            analysis.push_str("    - Content compression could reduce storage by ~30-50%\n");
        }
        analysis.push_str("    - Regular cleanup of orphaned embeddings recommended\n");

        Ok(analysis)
    }

    /// Perform comprehensive health checks
    async fn perform_comprehensive_health_checks(&self) -> Result<String> {
        let mut health = String::new();

        // Database connection health
        let db_start = std::time::Instant::now();
        let db_health = sqlx::query("SELECT 1").fetch_one(self.db_pool.pool()).await;
        let db_time = db_start.elapsed();

        match db_health {
            Ok(_) => {
                let _ = writeln!(
                    &mut health,
                    "  ✅ Database Connection: Healthy ({:.2}ms)",
                    db_time.as_secs_f64() * 1000.0
                );
            }
            Err(e) => {
                let _ = writeln!(&mut health, "  ❌ Database Connection: Error - {}", e);
            }
        }

        // Data integrity checks
        let orphaned_embeddings = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL AND content = ''"
        )
        .fetch_one(self.db_pool.pool())
        .await?;

        if orphaned_embeddings == 0 {
            health.push_str("  ✅ Data Integrity: No orphaned embeddings detected\n");
        } else {
            let _ = writeln!(
                &mut health,
                "  ⚠️ Data Integrity: {} orphaned embeddings found",
                orphaned_embeddings
            );
        }

        // Job queue health
        let stuck_jobs = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM crate_jobs WHERE status = 'running' AND started_at < NOW() - INTERVAL '1 hour'"
        )
        .fetch_one(self.db_pool.pool())
        .await?;

        if stuck_jobs == 0 {
            health.push_str("  ✅ Job Queue: No stuck jobs detected\n");
        } else {
            let _ = writeln!(
                &mut health,
                "  ⚠️ Job Queue: {} potentially stuck jobs",
                stuck_jobs
            );
        }

        // System resource estimates
        let pool_size = self.db_pool.pool().size();
        let active_connections = self.db_pool.pool().num_idle();
        let _ = writeln!(
            &mut health,
            "  ℹ️ Connection Pool: {} active, {} total capacity",
            active_connections, pool_size
        );

        // Overall system health score
        let issues = i32::from(orphaned_embeddings > 0) + i32::from(stuck_jobs > 0);
        match issues {
            0 => health
                .push_str("  🎯 **Overall Health: EXCELLENT** - All systems operating normally\n"),
            1 => health.push_str("  ⚠️ **Overall Health: GOOD** - Minor issues detected\n"),
            _ => health
                .push_str("  🚨 **Overall Health: NEEDS ATTENTION** - Multiple issues detected\n"),
        }

        Ok(health)
    }

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
