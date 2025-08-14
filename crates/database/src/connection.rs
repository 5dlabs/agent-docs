//! Database connection management

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DatabasePool {
    pool: PgPool,
}

impl DatabasePool {
    /// Create a new database pool from connection URL
    ///
    /// # Errors
    ///
    /// Returns an error if a connection to the database cannot be established
    /// within the configured timeout or the connection URL is invalid.
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;

        info!("Database connection established");

        Ok(Self { pool })
    }

    /// Get a reference to the underlying pool
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Test the database connection
    ///
    /// # Errors
    ///
    /// Returns an error if the connectivity check query fails.
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
