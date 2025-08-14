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

pub use connection::{DatabasePool, HealthCheckResult, PoolStatus, PoolMetricsSnapshot};
pub use migration_system::{DatabaseMigrationManager, MigrationInfo, MigrationStatus, MigrationHistory, SchemaValidationReport, MigrationStatusSummary};
pub use models::*;
pub use pool_config::{PoolConfig, PoolConfigBuilder};
pub use retry::{RetryConfig, RetryExecutor, DatabaseError};

/// Re-export commonly used types
pub use sqlx::{PgPool, Row};
pub use uuid::Uuid;
