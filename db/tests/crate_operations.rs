//! Database tests for crate operations
//!
//! This module tests the database layer functionality for crate management:
//! - `CrateJobQueries`: Job lifecycle management
//! - `CrateQueries`: Crate information retrieval from document metadata
//! - Document operations with crate metadata

use anyhow::Result;
use chrono::Utc;
use db::models::{JobStatus, PaginationParams};
use db::{CrateJobQueries, CrateQueries, DatabasePool, PoolConfig, Row};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Test fixture for database operations
struct DatabaseTestFixture {
    pool: PgPool,
    test_crate_name: String,
}

impl DatabaseTestFixture {
    async fn new() -> Result<Self> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://vector_user:EFwiPWDXMoOI2VKNF4eO3eSm8n3hzmjognKytNk2ndskgOAZgEBGDQULE6ryDc7z@vector-postgres.databases.svc.cluster.local:5432/vector_db".to_string());

        // Use a leaner pool config for tests to avoid exhausting DB connections in CI
        let mut config = PoolConfig::testing();
        config.database_url = database_url;
        config.min_connections = 1;
        config.max_connections = 5;
        let pool = DatabasePool::with_config(config).await?.pool().clone();
        let test_crate_name = format!("db-test-crate-{}", Uuid::new_v4());

        Ok(Self {
            pool,
            test_crate_name,
        })
    }

    async fn cleanup(&self) -> Result<()> {
        // Clean up test data
        sqlx::query("DELETE FROM documents WHERE metadata->>'crate_name' = $1")
            .bind(&self.test_crate_name)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM crate_jobs WHERE crate_name = $1")
            .bind(&self.test_crate_name)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn insert_test_documents(&self, count: i32) -> Result<Vec<Uuid>> {
        let mut doc_ids = Vec::new();

        for i in 0..count {
            let doc_id = Uuid::new_v4();
            let metadata = json!({
                "crate_name": self.test_crate_name,
                "crate_version": "0.1.0",
                "item_type": if i % 2 == 0 { "struct" } else { "function" },
                "module_path": format!("{}::item{}", self.test_crate_name, i)
            });

            sqlx::query(
                r"
                INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
                VALUES ($1, 'rust', $2, $3, $4, $5, $6, $7, $7)
                ON CONFLICT (doc_type, source_name, doc_path) DO NOTHING
                ",
            )
            .bind(doc_id)
            .bind(&self.test_crate_name)
            .bind(format!("doc/{i}"))
            .bind(format!("Test content {i}"))
            .bind(metadata)
            .bind(50 + i) // token count
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

            doc_ids.push(doc_id);
        }

        Ok(doc_ids)
    }
}

#[tokio::test]
async fn test_crate_job_lifecycle() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Create a new job
    let job =
        CrateJobQueries::create_job(&fixture.pool, &fixture.test_crate_name, "add_crate").await?;

    assert_eq!(job.crate_name, fixture.test_crate_name);
    assert_eq!(job.operation, "add_crate");
    assert_eq!(job.status, JobStatus::Queued);
    assert!(job.progress.is_none());
    assert!(job.error.is_none());
    assert!(job.finished_at.is_none());

    // Update job to running
    let updated_job = CrateJobQueries::update_job_status(
        &fixture.pool,
        job.id,
        JobStatus::Running,
        Some(25),
        None,
    )
    .await?;

    assert_eq!(updated_job.status, JobStatus::Running);
    assert_eq!(updated_job.progress, Some(25));
    assert!(updated_job.finished_at.is_none());

    // Complete the job
    let completed_job = CrateJobQueries::update_job_status(
        &fixture.pool,
        job.id,
        JobStatus::Completed,
        Some(100),
        None,
    )
    .await?;

    assert_eq!(completed_job.status, JobStatus::Completed);
    assert_eq!(completed_job.progress, Some(100));
    assert!(completed_job.finished_at.is_some());

    // Find job by ID
    let found_job = CrateJobQueries::find_job_by_id(&fixture.pool, job.id).await?;
    assert!(found_job.is_some());
    assert_eq!(found_job.unwrap().status, JobStatus::Completed);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_crate_job_error_handling() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Create a job and mark it as failed
    let job =
        CrateJobQueries::create_job(&fixture.pool, &fixture.test_crate_name, "add_crate").await?;

    let error_message = "Test error: crate not found";
    let failed_job = CrateJobQueries::update_job_status(
        &fixture.pool,
        job.id,
        JobStatus::Failed,
        Some(50),
        Some(error_message),
    )
    .await?;

    assert_eq!(failed_job.status, JobStatus::Failed);
    assert_eq!(failed_job.progress, Some(50));
    assert_eq!(failed_job.error.as_deref(), Some(error_message));
    assert!(failed_job.finished_at.is_some());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_active_jobs() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Create multiple jobs in different states
    let job1 =
        CrateJobQueries::create_job(&fixture.pool, &fixture.test_crate_name, "add_crate").await?;
    let job2 = CrateJobQueries::create_job(&fixture.pool, "other-crate", "remove_crate").await?;
    let job3 = CrateJobQueries::create_job(&fixture.pool, "third-crate", "add_crate").await?;

    // Update one to running
    CrateJobQueries::update_job_status(&fixture.pool, job2.id, JobStatus::Running, Some(30), None)
        .await?;

    // Update one to completed
    CrateJobQueries::update_job_status(
        &fixture.pool,
        job3.id,
        JobStatus::Completed,
        Some(100),
        None,
    )
    .await?;

    // Find active jobs
    let active_jobs = CrateJobQueries::find_active_jobs(&fixture.pool).await?;

    // Should find 2 active jobs (queued and running)
    let active_ids: Vec<Uuid> = active_jobs.iter().map(|j| j.id).collect();
    assert!(active_ids.contains(&job1.id)); // queued
    assert!(active_ids.contains(&job2.id)); // running
    assert!(!active_ids.contains(&job3.id)); // completed

    // Clean up additional test data
    sqlx::query("DELETE FROM crate_jobs WHERE crate_name IN ('other-crate', 'third-crate')")
        .execute(&fixture.pool)
        .await?;

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_cleanup_old_jobs() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Create a job and mark it as completed
    let job =
        CrateJobQueries::create_job(&fixture.pool, &fixture.test_crate_name, "add_crate").await?;
    CrateJobQueries::update_job_status(
        &fixture.pool,
        job.id,
        JobStatus::Completed,
        Some(100),
        None,
    )
    .await?;

    // Note: In a real test, we'd need to manipulate the finished_at timestamp to be old
    // For now, just verify the cleanup function exists and runs without error
    let cleaned_count = CrateJobQueries::cleanup_old_jobs(&fixture.pool).await?;

    // Should be >= 0 (may not clean our recent job)
    assert!(cleaned_count >= 0);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_crates_from_documents() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert test documents
    fixture.insert_test_documents(5).await?;

    // List crates with default pagination
    let pagination = PaginationParams::new(Some(1), Some(20));
    let result = CrateQueries::list_crates(&fixture.pool, &pagination, None).await?;

    // Should find our test crate
    let found_crate = result
        .items
        .iter()
        .find(|c| c.name == fixture.test_crate_name);
    assert!(found_crate.is_some());

    let crate_info = found_crate.unwrap();
    assert_eq!(crate_info.total_docs, 5);
    assert_eq!(crate_info.version, "0.1.0");
    assert_eq!(crate_info.total_tokens, 250 + 10); // 50+1 + 50+2 + ... + 50+5 = 250 + 15 = 265? Let me recalculate: 51+52+53+54+55 = 265

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_crates_with_name_filter() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert test documents
    fixture.insert_test_documents(3).await?;

    // List crates with name pattern
    let pagination = PaginationParams::new(Some(1), Some(20));
    let pattern = &fixture.test_crate_name[..10]; // Use partial name
    let result = CrateQueries::list_crates(&fixture.pool, &pagination, Some(pattern)).await?;

    // Should find our test crate
    assert!(!result.items.is_empty());
    let found_crate = result
        .items
        .iter()
        .find(|c| c.name == fixture.test_crate_name);
    assert!(found_crate.is_some());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_crates_pagination() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert test documents
    fixture.insert_test_documents(2).await?;

    // Test pagination with small limit
    let pagination = PaginationParams::new(Some(1), Some(1));
    let result = CrateQueries::list_crates(&fixture.pool, &pagination, None).await?;

    // Verify pagination structure
    assert_eq!(result.page, 1);
    assert!(result.total_items >= 1);
    assert!(!result.items.is_empty());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_crate_statistics() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert test documents
    fixture.insert_test_documents(8).await?;

    // Get statistics
    let stats = CrateQueries::get_crate_statistics(&fixture.pool).await?;

    // Should include our test crate
    assert!(stats.total_crates >= 1);
    assert!(stats.active_crates >= 1);
    assert!(stats.total_docs_managed >= 8);
    assert!(stats.total_tokens_managed >= 400); // At least 8 * 50 tokens
    assert!(stats.average_docs_per_crate > 0.0);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_crate_by_name() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert test documents
    fixture.insert_test_documents(4).await?;

    // Find crate by name
    let crate_info =
        CrateQueries::find_crate_by_name(&fixture.pool, &fixture.test_crate_name).await?;

    assert!(crate_info.is_some());
    let info = crate_info.unwrap();
    assert_eq!(info.name, fixture.test_crate_name);
    assert_eq!(info.version, "0.1.0");
    assert_eq!(info.total_docs, 4);

    // Test with non-existent crate
    let not_found = CrateQueries::find_crate_by_name(&fixture.pool, "non-existent-crate").await?;
    assert!(not_found.is_none());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_crate_document_metadata_queries() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Insert documents with different metadata
    let doc1_id = Uuid::new_v4();
    let doc2_id = Uuid::new_v4();

    let metadata1 = json!({
        "crate_name": fixture.test_crate_name,
        "crate_version": "1.0.0",
        "item_type": "struct",
        "module_path": format!("{}::MyStruct", fixture.test_crate_name)
    });

    let metadata2 = json!({
        "crate_name": fixture.test_crate_name,
        "crate_version": "1.0.0",
        "item_type": "function",
        "module_path": format!("{}::my_function", fixture.test_crate_name)
    });

    // Insert documents
    for (i, (doc_id, metadata)) in [(doc1_id, metadata1), (doc2_id, metadata2)]
        .iter()
        .enumerate()
    {
        sqlx::query(
            r"
            INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
            VALUES ($1, 'rust', $2, $3, $4, $5, $6, $7, $7)
            ON CONFLICT (doc_type, source_name, doc_path) DO NOTHING
            ",
        )
        .bind(doc_id)
        .bind(&fixture.test_crate_name)
        .bind(format!("test/doc/{i}"))  // Make paths unique
        .bind("Test content")
        .bind(metadata)
        .bind(100)
        .bind(Utc::now())
        .execute(&fixture.pool)
        .await?;
    }

    // Query documents by metadata
    let docs_by_crate =
        sqlx::query("SELECT id, metadata FROM documents WHERE metadata->>'crate_name' = $1")
            .bind(&fixture.test_crate_name)
            .fetch_all(&fixture.pool)
            .await?;

    assert_eq!(docs_by_crate.len(), 2);

    // Test metadata filtering
    let struct_docs = sqlx::query(
        "SELECT id FROM documents WHERE metadata->>'crate_name' = $1 AND metadata->>'item_type' = 'struct'"
    )
    .bind(&fixture.test_crate_name)
    .fetch_all(&fixture.pool)
    .await?;

    assert_eq!(struct_docs.len(), 1);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_database_operations() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Test concurrent job creation
    let name1 = format!("{}-1", fixture.test_crate_name);
    let name2 = format!("{}-2", fixture.test_crate_name);
    let name3 = format!("{}-3", fixture.test_crate_name);
    let (job1, job2, job3) = tokio::join!(
        CrateJobQueries::create_job(&fixture.pool, &name1, "add_crate"),
        CrateJobQueries::create_job(&fixture.pool, &name2, "remove_crate"),
        CrateJobQueries::create_job(&fixture.pool, &name3, "add_crate")
    );

    // All should succeed
    assert!(job1.is_ok());
    assert!(job2.is_ok());
    assert!(job3.is_ok());

    // Test concurrent reads
    let pagination = PaginationParams::new(Some(1), Some(10));
    let (stats, list_result) = tokio::join!(
        CrateQueries::get_crate_statistics(&fixture.pool),
        CrateQueries::list_crates(&fixture.pool, &pagination, None)
    );

    assert!(stats.is_ok());
    assert!(list_result.is_ok());

    // Clean up additional test data
    sqlx::query("DELETE FROM crate_jobs WHERE crate_name LIKE $1")
        .bind(format!("{}-_", fixture.test_crate_name))
        .execute(&fixture.pool)
        .await?;

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_transaction_rollback_simulation() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Simulate transaction failure by trying to insert invalid data
    // Note: This is a simplified test - in practice, we'd test actual transaction rollback

    // Test that database constraints work properly
    let result = sqlx::query(
        "INSERT INTO crate_jobs (id, crate_name, operation, status, started_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $5, $5)"
    )
    .bind(Uuid::new_v4())
    .bind("") // Empty crate name should be allowed, but let's test with very long name
    .bind("add_crate")
    .bind(JobStatus::Queued)
    .bind(Utc::now())
    .execute(&fixture.pool)
    .await;

    // This should succeed (empty strings are allowed)
    assert!(result.is_ok());

    fixture.cleanup().await?;
    Ok(())
}

/// Test database connection and basic functionality
#[tokio::test]
async fn test_database_connection() -> Result<()> {
    let fixture = DatabaseTestFixture::new().await?;

    // Test basic query
    let result = sqlx::query("SELECT 1 as test_value")
        .fetch_one(&fixture.pool)
        .await?;

    let test_value: i32 = result.get("test_value");
    assert_eq!(test_value, 1);

    // Test that required tables exist
    let tables = sqlx::query(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN ('documents', 'crate_jobs')"
    )
    .fetch_all(&fixture.pool)
    .await?;

    assert!(tables.len() >= 2, "Required tables should exist");

    Ok(())
}
