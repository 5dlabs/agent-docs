//! Advanced database migration system with versioning and rollback support
//!
//! This module provides a comprehensive migration framework that tracks version history,
//! validates schema integrity, and supports rollback capabilities for safe production deployment.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Row};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Migration metadata and version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationInfo {
    /// Unique migration identifier
    pub id: String,
    /// Migration version (semantic versioning)
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Migration SQL to apply
    pub up_sql: String,
    /// Rollback SQL (optional)
    pub down_sql: Option<String>,
    /// Dependencies on other migrations
    pub dependencies: Vec<String>,
    /// Checksum for integrity validation
    pub checksum: String,
}

/// Migration execution status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "migration_status", rename_all = "lowercase")]
pub enum MigrationStatus {
    /// Migration is pending execution
    Pending,
    /// Migration is currently running
    Running,
    /// Migration completed successfully
    Completed,
    /// Migration failed
    Failed,
    /// Migration was rolled back
    RolledBack,
}

/// Migration history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationHistory {
    pub id: Uuid,
    pub migration_id: String,
    pub version: String,
    pub status: MigrationStatus,
    pub applied_at: DateTime<Utc>,
    pub execution_time_ms: i64,
    pub error_message: Option<String>,
    pub applied_by: String,
}

/// Advanced migration manager with version control and rollback support
pub struct DatabaseMigrationManager {
    pool: PgPool,
    migrations: HashMap<String, MigrationInfo>,
}

impl DatabaseMigrationManager {
    /// Create a new migration manager
    ///
    /// # Errors
    ///
    /// Returns an error if the migration metadata table cannot be created.
    pub async fn new(pool: PgPool) -> Result<Self> {
        let manager = Self {
            pool,
            migrations: HashMap::new(),
        };

        manager.initialize_migration_tables().await?;
        Ok(manager)
    }

    /// Initialize migration metadata tables if they don't exist
    async fn initialize_migration_tables(&self) -> Result<()> {
        info!("Initializing migration metadata tables...");

        // Create migration status enum
        sqlx::query(
            r"
            DO $$ BEGIN
                CREATE TYPE migration_status AS ENUM ('pending', 'running', 'completed', 'failed', 'rolledback');
            EXCEPTION
                WHEN duplicate_object THEN null;
            END $$;
        ",
        )
        .execute(&self.pool)
        .await?;

        // Create migration history table (idempotent)
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS migration_history (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                migration_id VARCHAR(255) NOT NULL,
                version VARCHAR(50) NOT NULL,
                status migration_status NOT NULL,
                applied_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                execution_time_ms BIGINT NOT NULL DEFAULT 0,
                error_message TEXT,
                applied_by VARCHAR(255) NOT NULL DEFAULT 'system',
                checksum VARCHAR(64) NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
        ",
        )
        .execute(&self.pool)
        .await?;

        // Self-heal existing tables that may be missing new columns from earlier versions
        // Add columns if they do not exist
        sqlx::query(
            r"
            DO $$ BEGIN
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS migration_id VARCHAR(255);
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS version VARCHAR(50);
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS status migration_status DEFAULT 'completed';
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS applied_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS execution_time_ms BIGINT DEFAULT 0;
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS error_message TEXT;
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS applied_by VARCHAR(255) DEFAULT 'system';
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS checksum VARCHAR(64) DEFAULT '';
                EXCEPTION WHEN duplicate_column THEN NULL; END;
                BEGIN
                    ALTER TABLE migration_history ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;
                EXCEPTION WHEN duplicate_column THEN NULL; END;
            END $$;
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_migration_history_migration_id ON migration_history(migration_id)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_migration_history_version ON migration_history(version)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_migration_history_status ON migration_history(status)",
        )
        .execute(&self.pool)
        .await?;

        info!("Migration metadata tables initialized successfully");
        Ok(())
    }

    /// Register a migration for execution
    pub fn register_migration(&mut self, migration: MigrationInfo) {
        info!(
            "Registering migration: {} ({})",
            migration.id, migration.version
        );
        self.migrations.insert(migration.id.clone(), migration);
    }

    /// Get all pending migrations in dependency order
    ///
    /// # Errors
    ///
    /// Returns an error if dependency resolution fails or database queries fail.
    pub async fn get_pending_migrations(&self) -> Result<Vec<MigrationInfo>> {
        let applied_migrations = self.get_applied_migrations().await?;
        let applied_ids: std::collections::HashSet<String> = applied_migrations
            .into_iter()
            .map(|m| m.migration_id)
            .collect();

        let mut pending: Vec<MigrationInfo> = self
            .migrations
            .values()
            .filter(|m| !applied_ids.contains(&m.id))
            .cloned()
            .collect();

        // Sort by dependencies and version
        pending.sort_by(|a, b| {
            // First sort by dependencies
            if a.dependencies.contains(&b.id) {
                std::cmp::Ordering::Greater
            } else if b.dependencies.contains(&a.id) {
                std::cmp::Ordering::Less
            } else {
                // Then by version
                a.version.cmp(&b.version)
            }
        });

        Ok(pending)
    }

    /// Get all applied migrations
    async fn get_applied_migrations(&self) -> Result<Vec<MigrationHistory>> {
        let rows = sqlx::query(
            r"
            SELECT id, migration_id, version, status, applied_at,
                   execution_time_ms, error_message, applied_by
            FROM migration_history
            WHERE status IN ('completed', 'failed', 'rolledback')
            ORDER BY applied_at ASC
        ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut migrations = Vec::new();
        for row in rows {
            migrations.push(MigrationHistory {
                id: row.get("id"),
                migration_id: row.get("migration_id"),
                version: row.get("version"),
                status: row.get("status"),
                applied_at: row.get("applied_at"),
                execution_time_ms: row.get("execution_time_ms"),
                error_message: row.get("error_message"),
                applied_by: row.get("applied_by"),
            });
        }

        Ok(migrations)
    }

    /// Apply all pending migrations with transaction safety
    ///
    /// # Errors
    ///
    /// Returns an error if any migration fails or dependency validation fails.
    pub async fn apply_migrations(&self) -> Result<Vec<MigrationHistory>> {
        info!("Starting migration application process...");

        let pending = self.get_pending_migrations().await?;
        if pending.is_empty() {
            info!("No pending migrations to apply");
            return Ok(Vec::new());
        }

        info!("Found {} pending migrations to apply", pending.len());
        let mut applied = Vec::new();

        for migration in pending {
            info!(
                "Applying migration: {} ({})",
                migration.id, migration.version
            );

            match self.apply_single_migration(&migration).await {
                Ok(history) => {
                    info!(
                        "Successfully applied migration: {} in {}ms",
                        migration.id, history.execution_time_ms
                    );
                    applied.push(history);
                }
                Err(e) => {
                    error!("Failed to apply migration {}: {}", migration.id, e);

                    // Record the failure
                    self.record_migration_failure(&migration, &e.to_string())
                        .await?;

                    // Stop applying further migrations on failure
                    return Err(anyhow!("Migration {} failed: {}", migration.id, e));
                }
            }
        }

        info!("Successfully applied {} migrations", applied.len());
        Ok(applied)
    }

    /// Apply a single migration with transaction safety
    async fn apply_single_migration(&self, migration: &MigrationInfo) -> Result<MigrationHistory> {
        let start_time = std::time::Instant::now();

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Record migration start
        let history_id = self.record_migration_start(&mut tx, migration).await?;

        // Execute migration SQL
        match sqlx::query::<Postgres>(&migration.up_sql)
            .execute(&mut *tx)
            .await
        {
            Ok(_) => {
                let execution_time =
                    i64::try_from(start_time.elapsed().as_millis()).unwrap_or(i64::MAX);

                // Update migration as completed
                let history = self
                    .record_migration_completion(&mut tx, history_id, execution_time)
                    .await?;

                // Commit transaction
                tx.commit().await?;

                Ok(history)
            }
            Err(e) => {
                // Rollback transaction on failure
                tx.rollback().await?;
                Err(anyhow!("Migration SQL execution failed: {}", e))
            }
        }
    }

    /// Record migration start in history
    async fn record_migration_start(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        migration: &MigrationInfo,
    ) -> Result<Uuid> {
        let history_id = Uuid::new_v4();

        sqlx::query::<Postgres>(
            r"
            INSERT INTO migration_history (id, migration_id, version, status, checksum, applied_by)
            VALUES ($1, $2, $3, 'running', $4, $5)
        ",
        )
        .bind(history_id)
        .bind(&migration.id)
        .bind(&migration.version)
        .bind(&migration.checksum)
        .bind("migration_system")
        .execute(&mut **tx)
        .await?;

        Ok(history_id)
    }

    /// Record migration completion
    async fn record_migration_completion(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        history_id: Uuid,
        execution_time: i64,
    ) -> Result<MigrationHistory> {
        sqlx::query::<Postgres>(
            r"
            UPDATE migration_history
            SET status = 'completed', execution_time_ms = $2
            WHERE id = $1
        ",
        )
        .bind(history_id)
        .bind(execution_time)
        .execute(&mut **tx)
        .await?;

        // Fetch the updated record
        let row = sqlx::query::<Postgres>(
            r"
            SELECT id, migration_id, version, status, applied_at,
                   execution_time_ms, error_message, applied_by
            FROM migration_history
            WHERE id = $1
        ",
        )
        .bind(history_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(MigrationHistory {
            id: row.get("id"),
            migration_id: row.get("migration_id"),
            version: row.get("version"),
            status: row.get("status"),
            applied_at: row.get("applied_at"),
            execution_time_ms: row.get("execution_time_ms"),
            error_message: row.get("error_message"),
            applied_by: row.get("applied_by"),
        })
    }

    /// Record migration failure
    async fn record_migration_failure(&self, migration: &MigrationInfo, error: &str) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO migration_history (migration_id, version, status, error_message, checksum, applied_by)
            VALUES ($1, $2, 'failed', $3, $4, $5)
        ",
        )
        .bind(&migration.id)
        .bind(&migration.version)
        .bind(error)
        .bind(&migration.checksum)
        .bind("migration_system")
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Validate current schema integrity
    ///
    /// # Errors
    ///
    /// Returns an error if schema validation fails or required components are missing.
    pub async fn validate_schema(&self) -> Result<SchemaValidationReport> {
        info!("Validating database schema integrity...");

        let mut report = SchemaValidationReport {
            is_valid: true,
            issues: Vec::new(),
            extensions: HashMap::new(),
            tables: HashMap::new(),
            indexes: HashMap::new(),
        };

        // Check required extensions
        self.validate_extensions(&mut report).await?;

        // Check required tables
        self.validate_tables(&mut report).await?;

        // Check required indexes
        self.validate_indexes(&mut report).await?;

        // Check pgvector dimensions support
        self.validate_pgvector_dimensions(&mut report).await?;

        if report.issues.is_empty() {
            info!("Schema validation completed successfully");
        } else {
            warn!("Schema validation found {} issues", report.issues.len());
            for issue in &report.issues {
                warn!("Schema issue: {}", issue);
            }
        }

        Ok(report)
    }

    /// Validate required extensions
    async fn validate_extensions(&self, report: &mut SchemaValidationReport) -> Result<()> {
        let required_extensions = vec!["vector", "uuid-ossp"];

        for ext_name in required_extensions {
            let row = sqlx::query(
                "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = $1) as installed",
            )
            .bind(ext_name)
            .fetch_one(&self.pool)
            .await?;

            let installed: bool = row.get("installed");
            report.extensions.insert(ext_name.to_string(), installed);

            if !installed {
                report.is_valid = false;
                report
                    .issues
                    .push(format!("Required extension '{ext_name}' is not installed"));
            }
        }

        Ok(())
    }

    /// Validate required tables
    async fn validate_tables(&self, report: &mut SchemaValidationReport) -> Result<()> {
        let required_tables = vec!["documents", "document_sources", "migration_history"];

        for table_name in required_tables {
            let row = sqlx::query(
                r"
                SELECT EXISTS (
                    SELECT 1 FROM information_schema.tables
                    WHERE table_schema = 'public' AND table_name = $1
                ) as exists
                ",
            )
            .bind(table_name)
            .fetch_one(&self.pool)
            .await?;

            let exists: bool = row.get("exists");
            report.tables.insert(table_name.to_string(), exists);

            if !exists {
                report.is_valid = false;
                report
                    .issues
                    .push(format!("Required table '{table_name}' does not exist"));
            }
        }

        Ok(())
    }

    /// Validate required indexes
    async fn validate_indexes(&self, report: &mut SchemaValidationReport) -> Result<()> {
        let required_indexes = vec![
            "idx_documents_doc_type",
            "idx_documents_source_name",
            "idx_migration_history_migration_id",
        ];

        for index_name in required_indexes {
            let row = sqlx::query(
                "SELECT EXISTS(SELECT 1 FROM pg_indexes WHERE indexname = $1) as exists",
            )
            .bind(index_name)
            .fetch_one(&self.pool)
            .await?;

            let exists: bool = row.get("exists");
            report.indexes.insert(index_name.to_string(), exists);

            if !exists {
                report.is_valid = false;
                report
                    .issues
                    .push(format!("Required index '{index_name}' does not exist"));
            }
        }

        Ok(())
    }

    /// Validate pgvector extension supports 3072 dimensions
    async fn validate_pgvector_dimensions(
        &self,
        report: &mut SchemaValidationReport,
    ) -> Result<()> {
        // Check if the type `vector(3072)` is accepted by the server (OpenAI text-embedding-3-large)
        // Use NULL::vector(3072) to validate type support without requiring a 3072-length literal
        match sqlx::query("SELECT NULL::vector(3072) as test_vector")
            .fetch_one(&self.pool)
            .await
        {
            Ok(_) => {
                info!("pgvector type accepts 3072 dimensions (text-embedding-3-large compatible)");
            }
            Err(_) => {
                report.issues.push(
                    "pgvector may not support 3072-dimensional vectors on this instance. If you intend to use 3072-dim embeddings, consider upgrading the pgvector extension or switch to a 1536-dim model.".to_string()
                );
            }
        }

        Ok(())
    }

    /// Get migration status summary
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails.
    pub async fn get_migration_status(&self) -> Result<MigrationStatusSummary> {
        let applied = self.get_applied_migrations().await?;
        let pending = self.get_pending_migrations().await?;

        let completed_count = applied
            .iter()
            .filter(|m| matches!(m.status, MigrationStatus::Completed))
            .count();
        let failed_count = applied
            .iter()
            .filter(|m| matches!(m.status, MigrationStatus::Failed))
            .count();

        Ok(MigrationStatusSummary {
            total_registered: self.migrations.len(),
            completed: completed_count,
            failed: failed_count,
            pending: pending.len(),
            last_applied: applied.last().map(|m| m.applied_at),
        })
    }
}

/// Schema validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaValidationReport {
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub extensions: HashMap<String, bool>,
    pub tables: HashMap<String, bool>,
    pub indexes: HashMap<String, bool>,
}

/// Migration status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatusSummary {
    pub total_registered: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub last_applied: Option<DateTime<Utc>>,
}
