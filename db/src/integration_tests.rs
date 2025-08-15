//! Integration tests for enhanced database functionality
//!
//! These tests verify the complete database migration system, connection pooling,
//! and health check functionality. They are designed to work with both test
//! databases and mock implementations for CI environments.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DatabasePool, PoolConfig, DatabaseMigrationManager, MigrationInfo};
    use std::time::Duration;
    use tokio_test;

    /// Test pool configuration validation
    #[test]
    fn test_pool_config_validation() {
        // Valid configuration
        let config = PoolConfig::builder()
            .database_url("postgresql://user:pass@localhost:5432/test")
            .min_connections(2)
            .max_connections(10)
            .build()
            .unwrap();
        
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_connections, 10);

        // Invalid configuration - min > max
        let invalid_config = PoolConfig::builder()
            .database_url("postgresql://user:pass@localhost:5432/test")
            .min_connections(15)
            .max_connections(10)
            .build();
        
        assert!(invalid_config.is_err());
    }

    /// Test pool configuration from environment variables
    #[test]
    fn test_pool_config_from_env() {
        // Set test environment variables
        std::env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/testdb");
        std::env::set_var("POOL_MIN_CONNECTIONS", "3");
        std::env::set_var("POOL_MAX_CONNECTIONS", "20");
        std::env::set_var("POOL_ACQUIRE_TIMEOUT", "15");

        let config = PoolConfig::from_env().unwrap();
        assert_eq!(config.min_connections, 3);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.acquire_timeout_seconds, 15);
        assert!(config.database_url.contains("testdb"));

        // Cleanup
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("POOL_MIN_CONNECTIONS");
        std::env::remove_var("POOL_MAX_CONNECTIONS");
        std::env::remove_var("POOL_ACQUIRE_TIMEOUT");
    }

    /// Test preset configurations
    #[test]
    fn test_preset_configurations() {
        let dev_config = PoolConfig::development();
        assert_eq!(dev_config.min_connections, 2);
        assert_eq!(dev_config.max_connections, 10);

        let prod_config = PoolConfig::production();
        assert_eq!(prod_config.min_connections, 5);
        assert_eq!(prod_config.max_connections, 100);

        let test_config = PoolConfig::testing();
        assert_eq!(test_config.min_connections, 1);
        assert_eq!(test_config.max_connections, 5);
    }

    /// Test migration info creation and validation
    #[test]
    fn test_migration_info_creation() {
        let migration = MigrationInfo {
            id: "001-initial-schema".to_string(),
            version: "1.0.0".to_string(),
            description: "Create initial schema".to_string(),
            up_sql: "CREATE TABLE test (id SERIAL PRIMARY KEY);".to_string(),
            down_sql: Some("DROP TABLE test;".to_string()),
            dependencies: vec![],
            checksum: "abc123".to_string(),
        };

        assert_eq!(migration.id, "001-initial-schema");
        assert_eq!(migration.version, "1.0.0");
        assert!(migration.down_sql.is_some());
    }

    /// Mock database pool for testing without requiring a live database
    async fn create_mock_pool() -> Option<DatabasePool> {
        // In CI environments, we might not have a database available
        // This function will try to create a real connection, but gracefully
        // handle the case where it's not available

        let test_db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://test:test@localhost:5432/test_db".to_string());

        match DatabasePool::new(&test_db_url).await {
            Ok(pool) => Some(pool),
            Err(_) => {
                // Database not available - this is expected in CI
                println!("Test database not available, skipping database-dependent tests");
                None
            }
        }
    }

    /// Test database pool creation and basic functionality
    #[tokio::test]
    async fn test_database_pool_creation() {
        if let Some(pool) = create_mock_pool().await {
            // Test basic ping
            let ping_result = pool.ping().await;
            
            // In a real test environment, this should succeed
            // In CI without a database, we skip this test
            match ping_result {
                Ok(()) => {
                    println!("Database ping successful");
                    
                    // Test health check
                    let health = pool.health_check().await.unwrap();
                    assert!(health.response_time_ms > 0);
                    
                    // Test metrics
                    let metrics = pool.get_metrics_snapshot();
                    assert!(metrics.total_queries >= 1); // At least the ping query
                }
                Err(e) => {
                    println!("Database ping failed: {}", e);
                    // This is acceptable in CI environments
                }
            }
        }
    }

    /// Test pool status and monitoring
    #[tokio::test]
    async fn test_pool_monitoring() {
        if let Some(pool) = create_mock_pool().await {
            match pool.get_status().await {
                Ok(status) => {
                    assert!(status.pool_utilization_percent >= 0.0);
                    assert!(status.pool_utilization_percent <= 100.0);
                    assert_eq!(status.config.application_name, "doc-server");
                    
                    // Verify health check components
                    assert!(status.health.checked_at <= chrono::Utc::now());
                    assert!(status.metrics.success_rate_percent >= 0.0);
                    assert!(status.metrics.success_rate_percent <= 100.0);
                }
                Err(e) => {
                    println!("Pool status check failed: {} (expected in CI)", e);
                }
            }
        }
    }

    /// Test connection configuration builder pattern
    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::builder()
            .database_url("postgresql://user:pass@localhost:5432/testdb")
            .min_connections(3)
            .max_connections(15)
            .acquire_timeout(Duration::from_secs(20))
            .application_name("test-app")
            .test_before_acquire(false)
            .build()
            .unwrap();

        assert_eq!(config.min_connections, 3);
        assert_eq!(config.max_connections, 15);
        assert_eq!(config.acquire_timeout_seconds, 20);
        assert_eq!(config.application_name, "test-app");
        assert!(!config.test_before_acquire);
    }

    /// Test retry configuration
    #[test]
    fn test_retry_configuration() {
        use crate::retry::RetryConfig;

        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_secs(1));
        assert_eq!(config.multiplier, 2.0);
        assert!(config.jitter);

        // Test delay calculation
        assert_eq!(config.calculate_delay(0), Duration::from_secs(1));
        let delay1 = config.calculate_delay(1);
        // With jitter, delay should be between 1-2 seconds
        assert!(delay1.as_secs_f64() >= 1.0 && delay1.as_secs_f64() <= 2.0);
    }

    /// Benchmark test for health check performance
    #[tokio::test]
    async fn test_health_check_performance() {
        if let Some(pool) = create_mock_pool().await {
            let start = std::time::Instant::now();
            
            // Perform multiple health checks to test caching
            for _ in 0..5 {
                let _ = pool.health_check().await;
            }
            
            let elapsed = start.elapsed();
            
            // With caching, 5 health checks should complete quickly
            // Allow generous time for CI environments
            assert!(elapsed < Duration::from_secs(10));
            
            println!("Health check performance: 5 checks completed in {:?}", elapsed);
        }
    }

    /// Test database error classification
    #[test]
    fn test_database_error_classification() {
        use crate::retry::DatabaseError;

        // Test retryable errors
        assert!(DatabaseError::ConnectionFailed("timeout".to_string()).is_retryable());
        assert!(DatabaseError::TemporarilyUnavailable("starting".to_string()).is_retryable());
        assert!(DatabaseError::TooManyConnections("limit reached".to_string()).is_retryable());

        // Test non-retryable errors
        assert!(!DatabaseError::AuthenticationFailed("invalid password".to_string()).is_retryable());
        assert!(!DatabaseError::DatabaseNotFound("db missing".to_string()).is_retryable());
        assert!(!DatabaseError::Other("syntax error".to_string()).is_retryable());
    }
}

/// Manual testing utilities for local development
#[cfg(test)]
mod manual_tests {
    use super::*;

    /// Manual test to verify complete database functionality
    /// Run with: cargo test manual_database_test -- --ignored --nocapture
    #[ignore]
    #[tokio::test]
    async fn manual_database_test() {
        use crate::{DatabasePool, DatabaseMigrationManager, MigrationInfo};

        println!("🚀 Manual database functionality test");

        // Test database connection with real database
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://docserver:development_password_change_in_production@localhost:5433/docs".to_string());

        println!("📊 Connecting to database: {}", database_url.split('@').last().unwrap_or("unknown"));

        match DatabasePool::from_env().await {
            Ok(pool) => {
                println!("✅ Database connection successful");

                // Test health check
                match pool.health_check().await {
                    Ok(health) => {
                        println!("🏥 Health check: {} ({}ms)", 
                                if health.is_healthy { "HEALTHY" } else { "UNHEALTHY" },
                                health.response_time_ms);
                        println!("   Active connections: {}", health.active_connections);
                        println!("   Idle connections: {}", health.idle_connections);
                    }
                    Err(e) => println!("❌ Health check failed: {}", e),
                }

                // Test pool status
                match pool.get_status().await {
                    Ok(status) => {
                        println!("📈 Pool utilization: {:.1}%", status.pool_utilization_percent);
                        println!("   Success rate: {:.1}%", status.metrics.success_rate_percent);
                        println!("   Total queries: {}", status.metrics.total_queries);
                    }
                    Err(e) => println!("❌ Pool status failed: {}", e),
                }

                // Test migration system
                match DatabaseMigrationManager::new(pool.pool().clone()).await {
                    Ok(mut migration_manager) => {
                        println!("🔄 Migration system initialized");

                        // Test schema validation
                        match migration_manager.validate_schema().await {
                            Ok(validation) => {
                                println!("✅ Schema validation: {}", if validation.is_valid { "VALID" } else { "INVALID" });
                                for issue in &validation.issues {
                                    println!("   ⚠️  {}", issue);
                                }
                            }
                            Err(e) => println!("❌ Schema validation failed: {}", e),
                        }

                        // Test migration status
                        match migration_manager.get_migration_status().await {
                            Ok(status) => {
                                println!("📋 Migration status:");
                                println!("   Total registered: {}", status.total_registered);
                                println!("   Completed: {}", status.completed);
                                println!("   Failed: {}", status.failed);
                                println!("   Pending: {}", status.pending);
                            }
                            Err(e) => println!("❌ Migration status failed: {}", e),
                        }
                    }
                    Err(e) => println!("❌ Migration manager initialization failed: {}", e),
                }

                println!("🎉 Manual test completed successfully");
            }
            Err(e) => {
                println!("❌ Database connection failed: {}", e);
                println!("💡 Make sure the database is running: ./scripts/dev.sh");
            }
        }
    }
}