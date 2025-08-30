//! Background job queue for crate ingestion

#![allow(clippy::must_use_candidate)]

use anyhow::Result;
use db::{
    models::{CrateJob, JobStatus},
    queries::CrateJobQueries,
    DatabasePool,
};
use embed::client::EmbeddingClient;
use loader::loaders::RustLoader;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

/// Background job message types
#[derive(Debug, Clone)]
pub enum JobMessage {
    AddCrate {
        job_id: Uuid,
        crate_name: String,
        version: Option<String>,
    },
}

/// Background job processor for crate ingestion
/// Manages background job execution with async processing
#[derive(Clone)]
pub struct CrateJobProcessor {
    db_pool: DatabasePool,
    job_sender: Option<mpsc::UnboundedSender<JobMessage>>,
}

impl CrateJobProcessor {
    /// Create a new job processor without background worker
    pub fn new(db_pool: DatabasePool) -> Self {
        Self {
            db_pool,
            job_sender: None,
        }
    }

    /// Create a new job processor with background worker
    pub fn new_with_worker(
        db_pool: DatabasePool,
        embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    ) -> Self {
        let (job_sender, job_receiver) = mpsc::unbounded_channel();

        // Start background worker
        let worker_db_pool = db_pool.clone();
        tokio::spawn(async move {
            let mut worker =
                BackgroundJobWorker::new(worker_db_pool, embedding_client, job_receiver);
            worker.run().await;
        });

        Self {
            db_pool,
            job_sender: Some(job_sender),
        }
    }

    /// Get a reference to the database pool
    pub fn db_pool(&self) -> &DatabasePool {
        &self.db_pool
    }

    /// Enqueue a new crate ingestion job
    ///
    /// # Errors
    ///
    /// Returns an error if the job cannot be created in the database.
    pub async fn enqueue_add_crate_job(
        &self,
        crate_name: &str,
        version: Option<&str>,
    ) -> Result<Uuid> {
        let job = CrateJobQueries::create_job(self.db_pool.pool(), crate_name, "add_crate").await?;

        info!("Enqueued add_crate job {} for {}", job.id, crate_name);

        // If we have a background worker, send the job for processing
        if let Some(sender) = &self.job_sender {
            let message = JobMessage::AddCrate {
                job_id: job.id,
                crate_name: crate_name.to_string(),
                version: version.map(String::from),
            };

            if let Err(e) = sender.send(message) {
                error!("Failed to send job to background worker: {}", e);
            }
        }

        Ok(job.id)
    }

    /// Get job status by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_job_status(&self, job_id: Uuid) -> Result<Option<CrateJob>> {
        CrateJobQueries::find_job_by_id(self.db_pool.pool(), job_id).await
    }

    /// Update job status
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
        progress: Option<i32>,
        error: Option<&str>,
    ) -> Result<CrateJob> {
        CrateJobQueries::update_job_status(self.db_pool.pool(), job_id, status, progress, error)
            .await
    }

    /// Clean up old completed jobs
    ///
    /// # Errors
    ///
    /// Returns an error if the cleanup operation fails.
    pub async fn cleanup_old_jobs(&self) -> Result<i32> {
        CrateJobQueries::cleanup_old_jobs(self.db_pool.pool()).await
    }
}

/// Background worker for processing crate jobs
pub struct BackgroundJobWorker {
    db_pool: DatabasePool,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    job_receiver: mpsc::UnboundedReceiver<JobMessage>,
    rust_loader: RustLoader,
}

impl BackgroundJobWorker {
    /// Create a new background worker
    pub fn new(
        db_pool: DatabasePool,
        embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
        job_receiver: mpsc::UnboundedReceiver<JobMessage>,
    ) -> Self {
        Self {
            db_pool,
            embedding_client,
            job_receiver,
            rust_loader: RustLoader::new(),
        }
    }

    /// Run the background worker
    pub async fn run(&mut self) {
        info!("Starting background job worker");

        while let Some(message) = self.job_receiver.recv().await {
            match message {
                JobMessage::AddCrate {
                    job_id,
                    crate_name,
                    version,
                } => {
                    info!("Processing add_crate job {} for {}", job_id, crate_name);

                    if let Err(e) = self
                        .process_add_crate_job(job_id, &crate_name, version.as_deref())
                        .await
                    {
                        error!("Failed to process add_crate job {}: {}", job_id, e);

                        // Update job status to failed
                        if let Err(update_err) = CrateJobQueries::update_job_status(
                            self.db_pool.pool(),
                            job_id,
                            JobStatus::Failed,
                            Some(0),
                            Some(&e.to_string()),
                        )
                        .await
                        {
                            error!("Failed to update job status to failed: {}", update_err);
                        }
                    }
                }
            }
        }

        info!("Background job worker stopped");
    }

    /// Process add crate job
    #[allow(clippy::too_many_lines)]
    async fn process_add_crate_job(
        &mut self,
        job_id: Uuid,
        crate_name: &str,
        version: Option<&str>,
    ) -> Result<()> {
        use serde_json::json;

        // Update job status to running
        CrateJobQueries::update_job_status(
            self.db_pool.pool(),
            job_id,
            JobStatus::Running,
            Some(0),
            None,
        )
        .await?;

        // Load crate documentation
        info!("Starting ingestion for crate: {}", crate_name);
        let (crate_info, doc_pages) = self
            .rust_loader
            .load_crate_docs(crate_name, version)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load crate documentation: {}", e))?;

        // Update progress
        CrateJobQueries::update_job_status(
            self.db_pool.pool(),
            job_id,
            JobStatus::Running,
            Some(25),
            None,
        )
        .await?;

        info!(
            "Processing {} documentation pages for crate {}",
            doc_pages.len(),
            crate_name
        );

        // Update progress
        CrateJobQueries::update_job_status(
            self.db_pool.pool(),
            job_id,
            JobStatus::Running,
            Some(50),
            None,
        )
        .await?;

        let mut total_docs = 0;
        let mut total_tokens = 0i64;
        let batch_size = 10;

        // Process documents in batches
        for (batch_idx, chunk) in doc_pages.chunks(batch_size).enumerate() {
            let mut tx = self.db_pool.pool().begin().await?;

            for doc_page in chunk {
                // Create document record
                let document_id = uuid::Uuid::new_v4();
                let metadata = json!({
                    "crate_name": crate_info.name,
                    "crate_version": crate_info.newest_version,
                    "item_type": doc_page.item_type,
                    "module_path": doc_page.module_path,
                    "extracted_at": doc_page.extracted_at,
                    "source_url": doc_page.url
                });

                // Calculate token count (approximation)
                let token_count = doc_page.content.len() / 4; // Rough approximation
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
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

                // Generate and store embedding
                if !doc_page.content.is_empty() {
                    match self.embedding_client.embed(&doc_page.content).await {
                        Ok(embedding) => {
                            let embedding_vector = pgvector::Vector::from(embedding);

                            // Update document with embedding
                            sqlx::query("UPDATE documents SET embedding = $1 WHERE id = $2")
                                .bind(embedding_vector)
                                .bind(document_id)
                                .execute(&mut *tx)
                                .await?;
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
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let token_count_i64 = token_count as i64;
                total_tokens += token_count_i64;
            }

            // Commit batch
            tx.commit().await?;

            // Update progress
            let total_batches = doc_pages.len().div_ceil(batch_size);
            let progress = 50 + ((batch_idx + 1) * 40 / total_batches);
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let progress_i32 = progress as i32;
            CrateJobQueries::update_job_status(
                self.db_pool.pool(),
                job_id,
                JobStatus::Running,
                Some(progress_i32),
                None,
            )
            .await?;

            info!(
                "Processed batch {} of {} for crate {}",
                batch_idx + 1,
                total_batches,
                crate_name
            );
        }

        // Mark job as completed
        CrateJobQueries::update_job_status(
            self.db_pool.pool(),
            job_id,
            JobStatus::Completed,
            Some(100),
            None,
        )
        .await?;

        info!(
            "Successfully completed ingestion for crate {}: {} documents, {} tokens",
            crate_name, total_docs, total_tokens
        );

        Ok(())
    }
}
