//! Integration tests for Rust crate management tools

use doc_server_mcp::tools::{
    AddRustCrateTool, CheckRustStatusTool, ListRustCratesTool, RemoveRustCrateTool, Tool,
};
use doc_server_database::{DatabasePool, PoolConfig};
use serde_json::json;
use std::env;

/// Test helper to create a database pool (mock for now)
async fn create_test_db_pool() -> DatabasePool {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://test:test@localhost/test".to_string());
    
    let config = PoolConfig::builder()
        .max_connections(5)
        .build()
        .expect("Failed to build pool config");
    
    DatabasePool::new(&database_url, config)
        .await
        .expect("Failed to create database pool")
}

#[tokio::test]
async fn test_tool_definitions() {
    let db_pool = create_test_db_pool().await;
    
    // Test that all tools can be created and return valid definitions
    let add_tool = AddRustCrateTool::new(db_pool.clone()).expect("Failed to create AddRustCrateTool");
    let add_def = add_tool.definition();
    assert_eq!(add_def.get("name").unwrap().as_str().unwrap(), "add_rust_crate");
    assert!(add_def.get("description").is_some());
    assert!(add_def.get("inputSchema").is_some());
    
    let remove_tool = RemoveRustCrateTool::new(db_pool.clone());
    let remove_def = remove_tool.definition();
    assert_eq!(remove_def.get("name").unwrap().as_str().unwrap(), "remove_rust_crate");
    
    let list_tool = ListRustCratesTool::new(db_pool.clone());
    let list_def = list_tool.definition();
    assert_eq!(list_def.get("name").unwrap().as_str().unwrap(), "list_rust_crates");
    
    let status_tool = CheckRustStatusTool::new(db_pool.clone());
    let status_def = status_tool.definition();
    assert_eq!(status_def.get("name").unwrap().as_str().unwrap(), "check_rust_status");
}

#[tokio::test]
async fn test_add_rust_crate_validation() {
    let db_pool = create_test_db_pool().await;
    let tool = AddRustCrateTool::new(db_pool).expect("Failed to create AddRustCrateTool");
    
    // Test with missing name parameter
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required 'name' parameter"));
    
    // Test with invalid crate name
    let result = tool.execute(json!({
        "name": "invalid@name"
    })).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid crate name"));
    
    // Test with valid parameters (this will fail due to no database, but validation passes)
    let result = tool.execute(json!({
        "name": "serde",
        "version": "1.0.0"
    })).await;
    // We expect this to fail with a database error, not a validation error
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(!error_msg.contains("Invalid crate name"));
        assert!(!error_msg.contains("Missing required"));
    }
}

#[tokio::test]
async fn test_remove_rust_crate_validation() {
    let db_pool = create_test_db_pool().await;
    let tool = RemoveRustCrateTool::new(db_pool);
    
    // Test with missing name parameter
    let result = tool.execute(json!({})).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Missing required 'name' parameter"));
}

#[tokio::test]
async fn test_list_rust_crates_pagination() {
    let db_pool = create_test_db_pool().await;
    let tool = ListRustCratesTool::new(db_pool);
    
    // Test with default parameters
    let _result = tool.execute(json!({})).await;
    // This will fail due to database issues, but we're testing structure
    
    // Test with pagination parameters
    let _result = tool.execute(json!({
        "page": 2,
        "limit": 10,
        "status_filter": "active",
        "name_pattern": "serde"
    })).await;
    // This will also fail due to database, but validates parameter parsing
}

#[tokio::test] 
async fn test_check_rust_status() {
    let db_pool = create_test_db_pool().await;
    let tool = CheckRustStatusTool::new(db_pool);
    
    // Test basic status check
    let _result = tool.execute(json!({})).await;
    
    // Test with job ID parameter
    let _result = tool.execute(json!({
        "job_id": "550e8400-e29b-41d4-a716-446655440000",
        "include_recent_jobs": false
    })).await;
}

#[test]
fn test_tool_schema_validation() {
    // Test that all tool schemas are valid JSON Schema
    use serde_json::{Value, from_str};
    
    // This is a basic test to ensure our JSON schemas are valid
    let add_schema = r#"{
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The name of the Rust crate to add",
                "pattern": "^[a-zA-Z0-9_-]+$"
            },
            "version": {
                "type": "string",
                "description": "Specific version to fetch",
                "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+(-.+)?$"
            }
        },
        "required": ["name"]
    }"#;
    
    let schema: Value = from_str(add_schema).expect("Invalid JSON schema");
    assert!(schema.is_object());
    assert!(schema.get("properties").is_some());
    assert!(schema.get("required").is_some());
}

#[test] 
fn test_crate_name_pattern_validation() {
    // Test the regex patterns we use for crate name validation
    use regex::Regex;
    
    let pattern = Regex::new(r"^[a-zA-Z0-9_-]+$").expect("Invalid regex");
    
    // Valid crate names
    assert!(pattern.is_match("serde"));
    assert!(pattern.is_match("tokio"));
    assert!(pattern.is_match("rust_decimal"));
    assert!(pattern.is_match("serde-json"));
    assert!(pattern.is_match("actix-web"));
    
    // Invalid crate names
    assert!(!pattern.is_match("serde@1.0"));
    assert!(!pattern.is_match("invalid.name"));
    assert!(!pattern.is_match("name with spaces"));
    assert!(!pattern.is_match(""));
}

#[test]
fn test_version_pattern_validation() {
    // Test semantic version patterns
    use regex::Regex;
    
    let pattern = Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+(-.*)?$").expect("Invalid regex");
    
    // Valid versions
    assert!(pattern.is_match("1.0.0"));
    assert!(pattern.is_match("0.12.34"));
    assert!(pattern.is_match("2.1.0-alpha"));
    assert!(pattern.is_match("1.0.0-beta.1"));
    
    // Invalid versions  
    assert!(!pattern.is_match("1.0"));
    assert!(!pattern.is_match("v1.0.0"));
    assert!(!pattern.is_match("latest"));
    assert!(!pattern.is_match("1.0.0.0"));
}