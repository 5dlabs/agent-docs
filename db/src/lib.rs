//! Database layer for the Doc Server
//!
//! This crate provides enhanced database connection, schema management, and query operations
//! for the Doc Server using `PostgreSQL` with pgvector extension. Features include:
//!
//! - Production-grade connection pooling with retry logic
//! - Comprehensive health checks and monitoring
//! - Versioned migration system with rollback support
//! - Schema integrity validation
//! - Connection pool metrics and alerting

pub mod connection;
pub mod migration_system;
pub mod migrations;
pub mod models;
pub mod pool_config;
pub mod queries;
pub mod retry;

pub use connection::{DatabasePool, HealthCheckResult, PoolMetricsSnapshot, PoolStatus};
pub use migration_system::{
    DatabaseMigrationManager, MigrationHistory, MigrationInfo, MigrationStatus,
    MigrationStatusSummary, SchemaValidationReport,
};
pub use models::*;
pub use pool_config::{PoolConfig, PoolConfigBuilder};
pub use queries::{
    CrateJobQueries, CrateQueries, DocumentQueries, QueryPerformanceMetrics,
    QueryPerformanceMonitor,
};
pub use retry::{DatabaseError, RetryConfig, RetryExecutor};

/// Re-export commonly used types
pub use sqlx::{PgPool, Row};
pub use uuid::Uuid;
