//! Legacy database migrations (DEPRECATED)
//!
//! **NOTE: This module is deprecated. Use `DatabaseMigrationManager` from `migration_system` instead.**
//!
//! The comprehensive migration system provides:
//! - Version tracking and history
//! - Dependency management
//! - Rollback capabilities
//! - Schema validation
//! - Transaction safety

use anyhow::Result;
use sqlx::PgPool;
use tracing::warn;

/// Legacy database migration operations
///
/// **DEPRECATED**: Use `crate::migration_system::DatabaseMigrationManager` instead.
/// This struct is kept for backward compatibility only.
#[deprecated = "Use DatabaseMigrationManager for comprehensive migration management"]
pub struct Migrations;

#[allow(deprecated)]
impl Migrations {
    /// Run all pending migrations (DEPRECATED)
    ///
    /// **DEPRECATED**: Use `DatabaseMigrationManager::apply_migrations()` instead.
    /// This method provides basic schema setup only and lacks versioning,
    /// rollback, and validation features.
    ///
    /// # Errors
    ///
    /// Returns an error if executing any migration SQL statement fails or the
    /// database connection encounters an error during execution.
    ///
    /// # Migration to New System
    ///
    /// Replace this:
    /// ```ignore
    /// Migrations::run(&pool)?;
    /// ```
    ///
    /// With this:
    /// ```ignore
    /// let mut manager = DatabaseMigrationManager::new(pool).await?;
    /// register_core_migrations(&mut manager);  // Register your migrations
    /// manager.apply_migrations().await?;
    /// ```
    #[deprecated = "Use DatabaseMigrationManager::apply_migrations() for comprehensive migration management"]
    pub fn run(_pool: &PgPool) -> Result<()> {
        warn!("Migrations::run() is deprecated and no longer functional");
        warn!("The comprehensive DatabaseMigrationManager system is now used");
        warn!("This method now returns immediately to avoid conflicts");
        warn!("Migration is handled by the startup process using DatabaseMigrationManager");

        Ok(())
    }
}
