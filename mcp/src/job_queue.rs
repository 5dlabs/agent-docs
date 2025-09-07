//! Background job queue for crate ingestion

use anyhow::Result;
use db::{
    models::{CrateJob, JobStatus},
    queries::CrateJobQueries,
    DatabasePool,
};
use tracing::info;
use uuid::Uuid;

/// Simplified job processor for crate ingestion
/// For now, this just manages job creation and status tracking
/// The actual processing happens synchronously in the tools
#[derive(Clone)]
pub struct CrateJobProcessor {
    db_pool: DatabasePool,
}

impl CrateJobProcessor {
    /// Create a new job processor
    #[must_use]
    pub const fn new(db_pool: DatabasePool) -> Self {
        Self { db_pool }
    }

    /// Get a reference to the database pool
    #[must_use]
    pub const fn db_pool(&self) -> &DatabasePool {
        &self.db_pool
    }

    /// Enqueue a new crate ingestion job
    ///
    /// # Errors
    ///
    /// Returns an error if the job cannot be created in the database.
    pub async fn enqueue_add_crate_job(&self, crate_name: &str) -> Result<Uuid> {
        let job = CrateJobQueries::create_job(self.db_pool.pool(), crate_name, "add_crate").await?;

        info!("Enqueued add_crate job {} for {}", job.id, crate_name);
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
