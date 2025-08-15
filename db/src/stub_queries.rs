//! Stub implementations for crate management queries
//! These are used when the database schema is not available during compilation

use anyhow::Result;
use crate::models::{CrateInfo, CrateJob, CrateStatus, JobStatus};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Stub implementation for crate queries
pub struct CrateQueries;

impl CrateQueries {
    /// Insert a new crate record (stub)
    pub async fn insert_crate(
        _pool: &PgPool,
        _name: &str,
        _version: &str,
        _description: Option<&str>,
        _documentation_url: Option<&str>,
    ) -> Result<i32> {
        // Return a mock ID for now
        Ok(1)
    }

    /// Find crate by name (stub)  
    pub async fn find_crate_by_name(_pool: &PgPool, _name: &str) -> Result<Option<CrateInfo>> {
        // Return None for now - crate not found
        Ok(None)
    }

    /// Update crate status and statistics (stub)
    pub async fn update_crate_stats(
        _pool: &PgPool,
        _name: &str,
        _status: CrateStatus,
        _total_docs: i32,
        _total_tokens: i64,
    ) -> Result<()> {
        Ok(())
    }

    /// Delete crate and associated documents (stub)
    pub async fn delete_crate_cascade(_pool: &PgPool, _name: &str) -> Result<u64> {
        Ok(1) // One row affected
    }

    /// Soft delete crate (stub)
    pub async fn soft_delete_crate(_pool: &PgPool, _name: &str) -> Result<bool> {
        Ok(true) // Successfully marked inactive
    }

    /// List crates with pagination and filtering (stub)
    pub async fn list_crates(
        _pool: &PgPool,
        _page: i64,
        _limit: i64,
        _status_filter: Option<CrateStatus>,
        _name_pattern: Option<&str>,
    ) -> Result<(Vec<CrateInfo>, i64)> {
        // Return empty list for now
        Ok((Vec::new(), 0))
    }

    /// Get crate statistics (stub)
    pub async fn get_crate_statistics(
        _pool: &PgPool,
    ) -> Result<(i64, i64, i64, i64, f64, Option<DateTime<Utc>>)> {
        // Return zero statistics
        Ok((0, 0, 0, 0, 0.0, None))
    }
}

/// Stub implementation for job queries  
pub struct JobQueries;

impl JobQueries {
    /// Create a new job (stub)
    pub async fn create_job(
        _pool: &PgPool,
        _crate_name: &str,
        _operation: &str,
    ) -> Result<Uuid> {
        Ok(Uuid::new_v4())
    }

    /// Update job status and progress (stub)
    pub async fn update_job_status(
        _pool: &PgPool,
        _job_id: Uuid,
        _status: JobStatus,
        _progress: Option<i32>,
        _error: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }

    /// Get job by ID (stub)
    pub async fn get_job(_pool: &PgPool, _job_id: Uuid) -> Result<Option<CrateJob>> {
        // Return a mock job for demonstration
        Ok(Some(CrateJob {
            id: _job_id,
            crate_name: "test-crate".to_string(),
            operation: "add_crate".to_string(),
            status: JobStatus::Completed,
            progress: Some(100),
            error: None,
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }))
    }

    /// Get recent jobs with pagination (stub)
    pub async fn list_jobs(
        _pool: &PgPool,
        _limit: i64,
        _status_filter: Option<JobStatus>,
    ) -> Result<Vec<CrateJob>> {
        // Return empty list for now
        Ok(Vec::new())
    }

    /// Clean up old completed jobs (stub)
    pub async fn cleanup_old_jobs(_pool: &PgPool) -> Result<u64> {
        Ok(0) // No jobs cleaned up
    }
}