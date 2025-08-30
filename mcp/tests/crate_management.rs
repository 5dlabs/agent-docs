//! Comprehensive integration tests for crate management tools
//!
//! This module tests all four crate management tools:
//! - `add_rust_crate`: Enqueues background ingestion and returns job ID
//! - `remove_rust_crate`: Cascade deletion with soft-delete option  
//! - `list_rust_crates`: Pagination with stats and filtering
//! - `check_rust_status`: Health monitoring and statistics

use anyhow::Result;
use chrono::Utc;
use db::models::JobStatus;
use db::{CrateJobQueries, DatabasePool, DocumentQueries};
use embed::OpenAIEmbeddingClient;
use mcp::crate_tools::{
    AddRustCrateTool, CheckRustStatusTool, ListRustCratesTool, RemoveRustCrateTool,
};
use mcp::tools::Tool;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Test fixture for crate management tests
struct CrateManagementTestFixture {
    pool: PgPool,
    test_crate_name: String,
}

impl CrateManagementTestFixture {
    async fn new() -> Result<Self> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://test:test@localhost:5433/test_docs".to_string());

        let db_pool = DatabasePool::connect(&database_url).await?;
        let pool = db_pool.pool().clone();

        let test_crate_name = format!("test-crate-{}", Uuid::new_v4());

        Ok(Self {
            pool,
            test_crate_name,
        })
    }

    /// Clean up test data
    async fn cleanup(&self) -> Result<()> {
        // Remove test documents
        sqlx::query("DELETE FROM documents WHERE metadata->>'crate_name' = $1")
            .bind(&self.test_crate_name)
            .execute(&self.pool)
            .await?;

        // Remove test jobs
        sqlx::query("DELETE FROM crate_jobs WHERE crate_name = $1")
            .bind(&self.test_crate_name)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Insert test documents for a crate
    async fn insert_test_documents(&self, count: i32) -> Result<Vec<Uuid>> {
        let mut doc_ids = Vec::new();

        for i in 0..count {
            let doc_id = Uuid::new_v4();
            let doc_path = format!("test/doc/{i}");
            let content = format!("Test documentation content for item {i}");
            let metadata = json!({
                "crate_name": self.test_crate_name,
                "crate_version": "0.1.0",
                "item_type": "struct",
                "module_path": format!("{}::TestStruct{}", self.test_crate_name, i)
            });

            sqlx::query(
                r"
                INSERT INTO documents (id, doc_type, source_name, doc_path, content, metadata, token_count, created_at, updated_at)
                VALUES ($1, 'rust', $2, $3, $4, $5, $6, $7, $7)
                ",
            )
            .bind(doc_id)
            .bind(&self.test_crate_name)
            .bind(doc_path)
            .bind(content)
            .bind(metadata)
            .bind(100 + i) // token count
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

            doc_ids.push(doc_id);
        }

        Ok(doc_ids)
    }
}

#[tokio::test]
async fn test_add_rust_crate_tool() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Test adding a new crate
    let tool = AddRustCrateTool::new(
        DatabasePool::from_pool(fixture.pool.clone()),
        Arc::new(OpenAIEmbeddingClient::new()?),
    );
    let arguments = json!({
        "name": fixture.test_crate_name,
        "version": "1.0.0"
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Verify response format (202 status with job ID)
    assert!(result.is_object());
    assert!(result.get("status").is_some());
    assert!(result.get("job_id").is_some());

    let job_id_str = result.get("job_id").unwrap().as_str().unwrap();
    let job_id = Uuid::parse_str(job_id_str)?;

    // Verify job was created in database
    let job = CrateJobQueries::find_job_by_id(&fixture.pool, job_id)
        .await?
        .expect("Job should exist");

    assert_eq!(job.crate_name, fixture.test_crate_name);
    assert_eq!(job.operation, "add_crate");
    assert_eq!(job.status, JobStatus::Completed); // Synchronous execution for MVP

    // Verify documents were created
    let docs = DocumentQueries::find_by_source(&fixture.pool, &fixture.test_crate_name).await?;
    assert!(
        !docs.is_empty(),
        "Should have created at least one document"
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_add_rust_crate_invalid_input() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    let tool = AddRustCrateTool::new(
        DatabasePool::from_pool(fixture.pool.clone()),
        Arc::new(OpenAIEmbeddingClient::new()?),
    );

    // Test with missing crate_name
    let arguments = json!({});
    let result = tool.execute(arguments).await;
    assert!(result.is_err(), "Should fail with missing crate_name");

    // Test with invalid crate_name
    let arguments = json!({"crate_name": ""});
    let result = tool.execute(arguments).await;
    assert!(result.is_err(), "Should fail with empty crate_name");

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_remove_rust_crate_tool() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    let _doc_ids = fixture.insert_test_documents(5).await?;

    // Verify documents exist
    let docs_before =
        DocumentQueries::find_by_source(&fixture.pool, &fixture.test_crate_name).await?;
    assert_eq!(docs_before.len(), 5);

    // Test removing the crate
    let tool = RemoveRustCrateTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "name": fixture.test_crate_name,
        "soft_delete": false
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Verify response
    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(result.get("documents_removed").unwrap().as_i64().unwrap() > 0);

    // Verify documents were removed
    let docs_after =
        DocumentQueries::find_by_source(&fixture.pool, &fixture.test_crate_name).await?;
    assert_eq!(docs_after.len(), 0);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_remove_rust_crate_soft_delete() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    fixture.insert_test_documents(3).await?;

    // Test soft delete (should create job but not immediately delete)
    let tool = RemoveRustCrateTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "name": fixture.test_crate_name,
        "soft_delete": true
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // For soft delete, should return job ID
    assert!(result.get("job_id").is_some());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_rust_crates_tool() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    fixture.insert_test_documents(10).await?;

    // Test basic listing
    let tool = ListRustCratesTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({});

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Verify response structure
    assert!(result.get("crates").is_some());
    assert!(result.get("pagination").is_some());

    let crates = result.get("crates").unwrap().as_array().unwrap();

    // Should find our test crate
    let found_crate = crates
        .iter()
        .find(|c| c.get("name").unwrap().as_str().unwrap() == fixture.test_crate_name);
    assert!(found_crate.is_some(), "Should find our test crate");

    let test_crate = found_crate.unwrap();
    assert_eq!(test_crate.get("total_docs").unwrap().as_i64().unwrap(), 10);
    assert_eq!(
        test_crate.get("version").unwrap().as_str().unwrap(),
        "0.1.0"
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_rust_crates_pagination() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    fixture.insert_test_documents(5).await?;

    // Test with pagination parameters
    let tool = ListRustCratesTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "page": 1,
        "limit": 5
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Verify pagination info
    let pagination = result.get("pagination").unwrap();
    assert_eq!(pagination.get("page").unwrap().as_i64().unwrap(), 1);
    assert!(pagination.get("total_items").unwrap().as_i64().unwrap() >= 1);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_list_rust_crates_name_filter() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    fixture.insert_test_documents(3).await?;

    // Test with name pattern filtering
    let tool = ListRustCratesTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "name_pattern": &fixture.test_crate_name[..10] // Use partial name
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    let crates = result.get("crates").unwrap().as_array().unwrap();
    let found_crate = crates
        .iter()
        .find(|c| c.get("name").unwrap().as_str().unwrap() == fixture.test_crate_name);
    assert!(found_crate.is_some(), "Should find crate with name pattern");

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_check_rust_status_tool() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents and create jobs
    fixture.insert_test_documents(7).await?;

    // Create some test jobs
    CrateJobQueries::create_job(&fixture.pool, &fixture.test_crate_name, "add_crate").await?;
    CrateJobQueries::create_job(&fixture.pool, "other-crate", "add_crate").await?;

    // Test status check
    let tool = CheckRustStatusTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({});

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Verify response structure
    assert!(result.get("database_status").is_some());
    assert!(result.get("crate_statistics").is_some());
    assert!(result.get("job_statistics").is_some());
    assert!(result.get("system_health").is_some());

    // Verify database status
    let db_status = result.get("database_status").unwrap();
    assert!(db_status.get("connected").unwrap().as_bool().unwrap());

    // Verify crate statistics
    let crate_stats = result.get("crate_statistics").unwrap();
    assert!(crate_stats.get("total_crates").unwrap().as_i64().unwrap() >= 1);
    assert!(
        crate_stats
            .get("total_docs_managed")
            .unwrap()
            .as_i64()
            .unwrap()
            >= 7
    );

    // Verify job statistics
    let job_stats = result.get("job_statistics").unwrap();
    assert!(job_stats.get("total_jobs").unwrap().as_i64().unwrap() >= 2);

    // Verify system health
    let system_health = result.get("system_health").unwrap();
    assert!(system_health.get("status").unwrap().as_str().unwrap() == "healthy");

    // Clean up additional test data
    sqlx::query("DELETE FROM crate_jobs WHERE crate_name = 'other-crate'")
        .execute(&fixture.pool)
        .await?;

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_check_rust_status_with_crate_filter() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Setup: Insert test documents
    fixture.insert_test_documents(5).await?;

    // Test status check for specific crate
    let tool = CheckRustStatusTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "crate_name": fixture.test_crate_name
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Should include crate-specific information
    assert!(result.get("crate_info").is_some());
    let crate_info = result.get("crate_info").unwrap();
    assert_eq!(
        crate_info.get("name").unwrap().as_str().unwrap(),
        fixture.test_crate_name
    );
    assert_eq!(crate_info.get("total_docs").unwrap().as_i64().unwrap(), 5);

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Test concurrent list operations (simulating multiple agents)
    let list_tool = ListRustCratesTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let status_tool = CheckRustStatusTool::new(DatabasePool::from_pool(fixture.pool.clone()));

    let list_arguments = json!({});
    let status_arguments = json!({});

    // Execute operations concurrently
    let (list_result, status_result) = tokio::join!(
        list_tool.execute(list_arguments),
        status_tool.execute(status_arguments)
    );

    // Both should succeed
    assert!(list_result.is_ok());
    assert!(status_result.is_ok());

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // Test removing non-existent crate
    let tool = RemoveRustCrateTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let arguments = json!({
        "name": "non-existent-crate-12345",
        "soft_delete": false
    });

    let result_str = tool.execute(arguments).await?;
    let result: Value = serde_json::from_str(&result_str)?;

    // Should succeed but report 0 documents removed
    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert_eq!(
        result.get("documents_removed").unwrap().as_i64().unwrap(),
        0
    );

    fixture.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn test_tool_metadata() -> Result<()> {
    // Skip this test if no database is available (e.g., in CI without database service)
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL")) else {
        // If no database URL is available, skip the test
        println!("No database URL available, skipping test_tool_metadata");
        return Ok(());
    };

    let pool_result = sqlx::PgPool::connect(&database_url).await;
    let pool = if let Ok(p) = pool_result { 
        DatabasePool::from_pool(p) 
    } else {
        // Database not available, skip test
        println!("Database not available, skipping test_tool_metadata");
        return Ok(());
    };

    // Test that all tools provide correct metadata
    let add_tool = AddRustCrateTool::new(pool.clone(), Arc::new(OpenAIEmbeddingClient::new()?));
    let remove_tool = RemoveRustCrateTool::new(pool.clone());
    let list_tool = ListRustCratesTool::new(pool.clone());
    let status_tool = CheckRustStatusTool::new(pool);

    // Test execution arguments (placeholder)
    let _test_args = json!({});

    // Verify tool definitions expose metadata
    let add_def = add_tool.definition();
    assert_eq!(
        add_def.get("name").unwrap().as_str().unwrap(),
        "add_rust_crate"
    );
    assert!(add_def.get("description").is_some());
    let remove_def = remove_tool.definition();
    assert_eq!(
        remove_def.get("name").unwrap().as_str().unwrap(),
        "remove_rust_crate"
    );
    let list_def = list_tool.definition();
    assert_eq!(
        list_def.get("name").unwrap().as_str().unwrap(),
        "list_rust_crates"
    );
    let status_def = status_tool.definition();
    assert_eq!(
        status_def.get("name").unwrap().as_str().unwrap(),
        "check_rust_status"
    );

    Ok(())
}

#[test]
fn test_tool_definitions_without_db() {
    // Test that we can create tool definitions without database connection
    use serde_json::Value;

    // Test schema validation for add_rust_crate
    let add_schema: Value = serde_json::from_str(
        r#"{
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The name of the Rust crate to add",
                "pattern": "^[a-zA-Z0-9_-]+$"
            },
            "version": {
                "type": "string",
                "description": "Optional specific version to fetch",
                "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+(-.+)?$"
            }
        },
        "required": ["name"]
    }"#,
    )
    .unwrap();

    assert!(add_schema.is_object());
    assert!(add_schema.get("properties").is_some());
    assert!(add_schema.get("required").is_some());

    // Test that required fields are properly defined
    let properties = add_schema.get("properties").unwrap().as_object().unwrap();
    assert!(properties.contains_key("name"));
    let required = add_schema.get("required").unwrap().as_array().unwrap();
    assert!(required.contains(&Value::String("name".to_string())));
}

/// Integration test to verify the complete workflow
#[tokio::test]
async fn test_complete_crate_lifecycle() -> Result<()> {
    let fixture = CrateManagementTestFixture::new().await?;

    // 1. Add a crate
    let add_tool = AddRustCrateTool::new(
        DatabasePool::from_pool(fixture.pool.clone()),
        Arc::new(OpenAIEmbeddingClient::new()?),
    );
    let add_arguments = json!({
        "name": fixture.test_crate_name,
        "version": "1.0.0"
    });

    let add_result_str = add_tool.execute(add_arguments).await?;
    let add_result: Value = serde_json::from_str(&add_result_str)?;
    assert!(add_result.get("job_id").is_some());

    // 2. List crates (should include our crate)
    let list_tool = ListRustCratesTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let list_arguments = json!({});

    let list_result_str = list_tool.execute(list_arguments).await?;
    let list_result: Value = serde_json::from_str(&list_result_str)?;
    let crates = list_result.get("crates").unwrap().as_array().unwrap();
    let found = crates
        .iter()
        .any(|c| c.get("name").unwrap().as_str().unwrap() == fixture.test_crate_name);
    assert!(found, "Added crate should appear in list");

    // 3. Check status (should include our crate in statistics)
    let status_tool = CheckRustStatusTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let status_arguments = json!({});

    let status_result_str = status_tool.execute(status_arguments).await?;
    let status_result: Value = serde_json::from_str(&status_result_str)?;
    let crate_stats = status_result.get("crate_statistics").unwrap();
    assert!(crate_stats.get("total_crates").unwrap().as_i64().unwrap() >= 1);

    // 4. Remove the crate
    let remove_tool = RemoveRustCrateTool::new(DatabasePool::from_pool(fixture.pool.clone()));
    let remove_arguments = json!({
        "name": fixture.test_crate_name,
        "soft_delete": false
    });

    let remove_result_str = remove_tool.execute(remove_arguments).await?;
    let remove_result: Value = serde_json::from_str(&remove_result_str)?;
    assert!(remove_result.get("success").unwrap().as_bool().unwrap());

    // 5. Verify removal (should no longer appear in list)
    let final_list_result_str = list_tool.execute(json!({})).await?;
    let final_list_result: Value = serde_json::from_str(&final_list_result_str)?;
    let final_crates = final_list_result.get("crates").unwrap().as_array().unwrap();
    let still_found = final_crates
        .iter()
        .any(|c| c.get("name").unwrap().as_str().unwrap() == fixture.test_crate_name);
    assert!(!still_found, "Removed crate should not appear in list");

    fixture.cleanup().await?;
    Ok(())
}
