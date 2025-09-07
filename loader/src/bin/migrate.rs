#!/usr/bin/env cargo
//! Data migration CLI for doc-server

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use db::models::DocType;
use embed::{EmbeddingClient, OpenAIEmbeddingClient};
use loader::migration::{MigrationConfig, MigrationPipeline, MigrationType, ValidationLevel};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, Level};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "migrate")]
#[command(about = "Data migration tool for doc-server")]
#[command(version = "0.1.0")]
struct MigrateCli {
    #[command(subcommand)]
    command: MigrateCommand,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// `OpenAI` API key for embeddings
    #[arg(long, env = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Execute full migration
    Full {
        /// Number of parallel workers
        #[arg(long, default_value = "4")]
        parallel: usize,

        /// Enable dry-run mode (no database writes)
        #[arg(long)]
        dry_run: bool,

        /// Maximum documents to process (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_documents: usize,

        /// Batch size for processing
        #[arg(long, default_value = "100")]
        batch_size: usize,

        /// Source data paths (format: type=path)
        #[arg(long, value_parser = parse_source_path)]
        source_path: Vec<(DocType, PathBuf)>,
    },
    /// Validate existing data
    Validate {
        /// Enable repair mode
        #[arg(long)]
        repair: bool,

        /// Validation level
        #[arg(long, default_value = "full")]
        level: String,
    },
    /// Rollback migration batch
    Rollback {
        /// Batch ID to rollback
        batch_id: String,
    },
    /// Resume migration from checkpoint
    Resume {
        /// Checkpoint ID to resume from
        checkpoint_id: String,
    },
    /// Show migration status and history
    Status,
}

fn parse_source_path(s: &str) -> Result<(DocType, PathBuf), String> {
    let (type_str, path_str) = s
        .split_once('=')
        .ok_or_else(|| "Source path must be in format 'type=path'".to_string())?;

    // With dynamic DocType as newtype, any type_str is valid
    let doc_type = DocType::from(type_str.to_lowercase());

    Ok((doc_type, PathBuf::from(path_str)))
}

async fn handle_full(
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    parallel: usize,
    dry_run: bool,
    max_documents: usize,
    batch_size: usize,
    source_paths: HashMap<DocType, PathBuf>,
) -> Result<()> {
    let config = MigrationConfig {
        parallel_workers: parallel,
        batch_size,
        max_documents,
        dry_run,
        validation_level: ValidationLevel::Full,
        source_paths,
        enable_checkpoints: true,
        checkpoint_frequency: 10,
    };

    let pipeline = MigrationPipeline::new(db_pool, embedding_client, config);

    match pipeline.execute_migration(MigrationType::Full).await {
        Ok(result) => {
            info!("Migration completed successfully!");
            info!(
                "Processed {} documents in {:?}",
                result.state.processed_documents, result.duration
            );
            info!("Throughput: {:.2} docs/min", result.throughput);
            info!(
                "Error rate: {:.2}%",
                result.performance_metrics.error_rate_percent
            );
        }
        Err(e) => {
            error!("Migration failed: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_validate(
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    repair: bool,
    level: String,
) -> Result<()> {
    let validation_level = match level.as_str() {
        "none" => ValidationLevel::None,
        "basic" => ValidationLevel::Basic,
        "full" => ValidationLevel::Full,
        _ => return Err(anyhow::anyhow!("Invalid validation level: {}", level)),
    };

    let config = MigrationConfig {
        validation_level,
        dry_run: !repair,
        ..Default::default()
    };

    let pipeline = MigrationPipeline::new(db_pool, embedding_client, config);

    match pipeline
        .execute_migration(MigrationType::ValidateOnly)
        .await
    {
        Ok(result) => {
            info!("Validation completed!");
            let report = &result.validation_report;
            info!("Total documents: {}", report.total_documents);
            info!("Validated: {}", report.validated_documents);
            info!("Failed validations: {}", report.failed_validations.len());
            info!("Checksum matches: {}", report.checksum_matches);
            info!("Schema violations: {}", report.schema_violations.len());

            if !report.failed_validations.is_empty() || !report.schema_violations.is_empty() {
                error!("Validation found issues!");
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Validation failed: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

fn handle_rollback(
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    batch_id: &str,
) -> Result<()> {
    let batch_uuid = Uuid::parse_str(batch_id).context("Invalid batch ID format")?;

    let config = MigrationConfig::default();
    let pipeline = MigrationPipeline::new(db_pool, embedding_client, config);

    match pipeline.rollback_batch(batch_uuid) {
        Ok(()) => {
            info!("Batch {} rolled back successfully", batch_id);
        }
        Err(e) => {
            error!("Rollback failed: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_resume(
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
    checkpoint_id: String,
) -> Result<()> {
    let checkpoint_uuid =
        Uuid::parse_str(&checkpoint_id).context("Invalid checkpoint ID format")?;

    let config = MigrationConfig::default();
    let pipeline = MigrationPipeline::new(db_pool, embedding_client, config);

    match pipeline
        .execute_migration(MigrationType::Resume {
            checkpoint_id: checkpoint_uuid,
        })
        .await
    {
        Ok(result) => {
            info!("Migration resumed and completed successfully!");
            info!(
                "Processed {} documents in {:?}",
                result.state.processed_documents, result.duration
            );
            info!("Throughput: {:.2} docs/min", result.throughput);
        }
        Err(e) => {
            error!("Resume migration failed: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn handle_status(
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient + Send + Sync>,
) -> Result<()> {
    let config = MigrationConfig::default();
    let pipeline = MigrationPipeline::new(db_pool, embedding_client, config);

    match pipeline.get_migration_history().await {
        Ok(history) => {
            info!("Migration History:");
            for state in history.iter().take(10) {
                // Show last 10
                info!(
                    "ID: {} | Type: {:?} | Status: {:?} | Started: {} | Processed: {}",
                    state.id,
                    state.migration_type,
                    state.status,
                    state.started_at.format("%Y-%m-%d %H:%M:%S"),
                    state.processed_documents
                );
            }

            if history.is_empty() {
                info!("No migration history found");
            }
        }
        Err(e) => {
            error!("Failed to get migration status: {:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = MigrateCli::parse();

    // Initialize database connection
    let database_url = args
        .database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("DATABASE_URL is required (pass --database-url or set DATABASE_URL)")
        })?;

    info!("Connecting to configured database (URL redacted)");
    let db_pool = Arc::new(
        PgPool::connect(&database_url)
            .await
            .context("Failed to connect to database")?,
    );

    // Initialize embedding client
    let embedding_client: Arc<dyn EmbeddingClient + Send + Sync> =
        Arc::new(OpenAIEmbeddingClient::new().context("Failed to create embedding client")?);

    match args.command {
        MigrateCommand::Full {
            parallel,
            dry_run,
            max_documents,
            batch_size,
            source_path,
        } => {
            let mut source_paths = HashMap::new();
            for (doc_type, path) in source_path {
                source_paths.insert(doc_type, path);
            }
            handle_full(
                db_pool,
                embedding_client,
                parallel,
                dry_run,
                max_documents,
                batch_size,
                source_paths,
            )
            .await?;
        }
        MigrateCommand::Validate { repair, level } => {
            handle_validate(db_pool, embedding_client, repair, level).await?;
        }
        MigrateCommand::Rollback { batch_id } => {
            handle_rollback(db_pool, embedding_client, &batch_id)?;
        }
        MigrateCommand::Resume { checkpoint_id } => {
            handle_resume(db_pool, embedding_client, checkpoint_id).await?;
        }
        MigrateCommand::Status => {
            handle_status(db_pool, embedding_client).await?;
        }
    }

    Ok(())
}
