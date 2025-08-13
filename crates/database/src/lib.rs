//! Database layer for the Doc Server
//!
//! This crate provides database connection, schema management, and query operations
//! for the Doc Server using PostgreSQL with pgvector extension.

pub mod connection;
pub mod migrations;
pub mod models;
pub mod queries;

pub use connection::DatabasePool;
pub use models::*;

/// Re-export commonly used types
pub use sqlx::{PgPool, Row};
pub use uuid::Uuid;
