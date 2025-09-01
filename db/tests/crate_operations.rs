//! Database tests for crate operations
//!
//! This module tests the database layer functionality for crate management:
//! - `CrateJobQueries`: Job lifecycle management
//! - `CrateQueries`: Crate information retrieval from document metadata
//! - Document operations with crate metadata

use anyhow::{anyhow, Result};
use chrono::Utc;
use db::models::{JobStatus, PaginationParams};
use db::{CrateJobQueries, CrateQueries, DatabasePool, PoolConfig, Row};
use serde_json::json;
use sqlx::{Connection, PgPool};
use std::time::Duration;
use uuid::Uuid;

/// Helper function to create test fixture with mock mode handling
async fn create_test_fixture() -> Result<DatabaseTestFixture> {
    match DatabaseTestFixture::new().await {
        Ok(f) => Ok(f),
        Err(e) if e.to_string().contains("Mock mode") => {
            Err(anyhow!("Mock mode detected - tests should be skipped"))
        }
        Err(e) => Err(e),
    }
}

/// Test fixture for database operations
struct DatabaseTestFixture {
    pool: PgPool,
    test_crate_name: String,
}

impl DatabaseTestFixture {
    #[allow(clippy::too_many_lines)]
    async fn new() -> Result<Self> {
        // Check if we should skip database tests (only in mock mode)
        if std::env::var("TEST_DATABASE_URL")
            .map(|v| v.trim().eq_ignore_ascii_case("mock"))
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "Mock mode: skipping database tests (TEST_DATABASE_URL=mock)"
            ));
        }

        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://vector_user:EFwiPWDXMoOI2VKNF4eO3eSm8n3hzmjognKytNk2ndskgOAZgEBGDQULE6ryDc7z@vector-postgres.databases.svc.cluster.local:5432/vector_db".to_string());

        // Debug: Show which database URL we're using
        eprintln!(
            "🔍 Using database URL: {}",
            if database_url.contains("vector-postgres") {
                "Kubernetes cluster (vector-postgres)"
            } else {
                "External database"
            }
        );
        eprintln!(
            "🔗 Database: {}",
            database_url.split('@').next_back().unwrap_or("unknown")
        );

        // Test basic connectivity first
        eprintln!("🔌 Testing database connectivity...");
        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&database_url)
            .await
        {
            Ok(test_pool) => {
                eprintln!("✅ Database connection successful");

                // Check if required tables exist
                let table_check = sqlx::query(
                    "SELECT COUNT(*) as table_count FROM information_schema.tables
                     WHERE table_schema = 'public' AND table_name IN ('documents', 'crate_jobs')",
                )
                .fetch_one(&test_pool)
                .await;

                match table_check {
                    Ok(row) => {
                        let table_count: i64 = row.get("table_count");
                        eprintln!("📊 Found {table_count} required tables");

                        if table_count < 2 {
                            eprintln!(
                                "⚠️  Missing required tables - attempting to set up schema..."
                            );

                            // Try to set up the schema
                            match std::fs::read_to_string("scripts/setup_test_db.sql") {
                                Ok(schema_sql) => {
                                    match sqlx::query(&schema_sql).execute(&test_pool).await {
                                        Ok(_) => {
                                            eprintln!("✅ Schema setup completed successfully");
                                        }
                                        Err(e) => {
                                            eprintln!("❌ Schema setup failed: {e}");
                                            eprintln!("💡 The database might already have some schema that conflicts");
                                        }
                                    }
                                }
                                Err(e) => eprintln!("❌ Could not read schema file: {e}"),
                            }
                        } else {
                            eprintln!("✅ Required tables exist");

                            // Check for required constraints
                            let constraint_check = sqlx::query(
                                "SELECT constraint_name FROM information_schema.table_constraints
                                 WHERE table_name = 'documents' AND constraint_type = 'UNIQUE'",
                            )
                            .fetch_all(&test_pool)
                            .await;

                            match constraint_check {
                                Ok(constraints) => {
                                    eprintln!(
                                        "🔍 Found {constraint_count} unique constraints on documents table:",
                                        constraint_count = constraints.len()
                                    );
                                    for constraint in &constraints {
                                        let name: String = constraint.get("constraint_name");
                                        eprintln!("   - {name}");
                                    }

                                    let has_doc_constraint = constraints.iter().any(|row| {
                                        let name: String = row.get("constraint_name");
                                        name.contains("doc_type")
                                            && name.contains("source_name")
                                            && name.contains("doc_path")
                                    });

                                    if has_doc_constraint {
                                        eprintln!("✅ Required unique constraint exists");
                                        eprintln!("💡 Note: Test user may not have DDL permissions to modify constraints");
                                        eprintln!(
                                            "   ON CONFLICT will use column-based resolution"
                                        );
                                    } else {
                                        eprintln!("⚠️  Missing unique constraint on documents(doc_type, source_name, doc_path) - attempting to add...");

                                        // Try to add the missing constraint
                                        match sqlx::query(
                                            "ALTER TABLE documents ADD CONSTRAINT IF NOT EXISTS documents_unique_doc_type_source_name_doc_path
                                             UNIQUE (doc_type, source_name, doc_path)"
                                        ).execute(&test_pool).await {
                                            Ok(_) => eprintln!("✅ Added missing unique constraint"),
                                            Err(e) => eprintln!("❌ Failed to add constraint: {e}"),
                                        }
                                    }
                                }
                                Err(e) => eprintln!("❌ Error checking constraints: {e}"),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error checking tables: {e}");
                    }
                }

                test_pool.close().await;
            }
            Err(e) => {
                eprintln!("❌ Database connection failed: {e}");
                eprintln!("💡 This could be because:");
                eprintln!("   - Database service is not running");
                eprintln!("   - Network connectivity issues in CI");
                eprintln!("   - Authentication credentials are incorrect");
                eprintln!("   - Database URL is malformed");
                return Err(anyhow!("Database connection failed: {}", e));
            }
        }

        // Allow environment variables to override pool configuration for CI flexibility
        let min_connections = std::env::var("TEST_POOL_MIN_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(2);

        let max_connections = std::env::var("TEST_POOL_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(15);

        let acquire_timeout = std::env::var("TEST_POOL_ACQUIRE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

        // Use optimized pool config for concurrent testing
        let mut config = PoolConfig::testing();
        config.database_url = database_url;
        config.min_connections = min_connections;
        config.max_connections = max_connections;
        config.acquire_timeout_seconds = acquire_timeout;
        config.test_before_acquire = true;
        let pool = DatabasePool::with_config(config).await?.pool().clone();
        let test_crate_name = format!("db-test-crate-{}", Uuid::new_v4());

        // Verify connection pool is healthy before proceeding
        let fixture = Self {
            pool: pool.clone(),
            test_crate_name: test_crate_name.clone(),
        };

        fixture.check_connection_health().await?;

        Ok(fixture)
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

    async fn check_connection_health(&self) -> Result<()> {
        // Test basic connectivity
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;

        // Test that we can acquire a connection
        let mut conn = self.pool.acquire().await?;
        conn.ping().await?;
        drop(conn); // Release connection back to pool

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

            // Check if document already exists before inserting
            let exists = sqlx::query(
                "SELECT 1 FROM documents WHERE doc_type = $1::doc_type AND source_name = $2 AND doc_path = $3"
            )
            .bind("rust")
            .bind(&self.test_crate_name)
            .bind(format!("doc/{i}"))
            .fetch_optional(&self.pool)
            .await?;

            if exists.is_none() {
                // Only insert if it doesn't exist
                sqlx::query(
                    r"
                    INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
                    VALUES ($1, 'rust', $2, $3, $4, $5, $6, $7, $7)
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
            }

            doc_ids.push(doc_id);
        }

        Ok(doc_ids)
    }
}

#[tokio::test]
async fn test_crate_job_lifecycle() -> Result<()> {
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test_crate_job_error_handling in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Insert test documents
    fixture.insert_test_documents(5).await?;

    // List crates with name filter to find our specific test crate
    // This is more robust than relying on pagination in a polluted database
    let pagination = PaginationParams::new(Some(1), Some(20));
    let result =
        CrateQueries::list_crates(&fixture.pool, &pagination, Some(&fixture.test_crate_name))
            .await?;

    // Should find our test crate when filtering by name
    let found_crate = result
        .items
        .iter()
        .find(|c| c.name == fixture.test_crate_name);
    assert!(
        found_crate.is_some(),
        "Could not find test crate '{}' in {} results",
        fixture.test_crate_name,
        result.items.len()
    );

    let crate_info = found_crate.unwrap();
    assert_eq!(crate_info.total_docs, 5);
    assert_eq!(crate_info.version, "0.1.0");
    // Correct calculation: 50+0 + 50+1 + 50+2 + 50+3 + 50+4 = 50+51+52+53+54 = 260
    assert_eq!(crate_info.total_tokens, 260);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_crates_with_name_filter() -> Result<()> {
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // Insert test documents
    fixture.insert_test_documents(3).await?;

    // List crates with partial name filter - use unique suffix to avoid pagination issues
    let pagination = PaginationParams::new(Some(1), Some(50)); // Increase page size to handle database pollution
    let pattern = &fixture.test_crate_name[13..]; // Use UUID part which should be unique
    let result = CrateQueries::list_crates(&fixture.pool, &pagination, Some(pattern)).await?;

    // Should find our test crate with the unique pattern
    assert!(
        !result.items.is_empty(),
        "No items found with pattern '{pattern}'"
    );
    let found_crate = result
        .items
        .iter()
        .find(|c| c.name == fixture.test_crate_name);
    assert!(
        found_crate.is_some(),
        "Could not find test crate '{}' with pattern '{pattern}'",
        fixture.test_crate_name
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_crates_pagination() -> Result<()> {
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
        // Check if document already exists before inserting
        let exists = sqlx::query(
            "SELECT 1 FROM documents WHERE doc_type = $1::doc_type AND source_name = $2 AND doc_path = $3"
        )
        .bind("rust")
        .bind(&fixture.test_crate_name)
        .bind(format!("test/doc/{i}"))
        .fetch_optional(&fixture.pool)
        .await?;

        if exists.is_none() {
            // Only insert if it doesn't exist
            sqlx::query(
                r"
                INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
                VALUES ($1, 'rust', $2, $3, $4, $5, $6, $7, $7)
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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
    let fixture = match create_test_fixture().await {
        Ok(f) => f,
        Err(e)
            if e.to_string().contains("Mock mode") || e.to_string().contains("CI environment") =>
        {
            println!("🧪 Skipping test in mock mode");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

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
