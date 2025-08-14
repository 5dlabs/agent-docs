//! HTTP server binary for the Doc Server
//!
//! This binary provides the main HTTP/SSE endpoint for MCP communication.

use anyhow::Result;
use doc_server_database::{
    DatabaseMigrationManager, DatabasePool, MigrationInfo, QueryPerformanceMonitor,
};
use doc_server_mcp::McpServer;
use dotenvy::dotenv;
use std::env;
use tokio::signal;
use tracing::{error, info, warn};
//use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Check for special operation modes
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--health-check" | "--version" => {
                // For health check, just exit successfully to indicate the binary is working
                println!("doc-server v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--migrate-only" => {
                // Run migrations only and exit (for K8s migration jobs)
                return run_migrations_only().await;
            }
            _ => {
                // Continue with normal startup
            }
        }
    }

    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "info,doc_server=debug".to_string()),
        )
        .init();

    info!("Starting Doc Server HTTP server...");

    // Get configuration from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    // Initialize database
    let db_pool = DatabasePool::new(&database_url).await?;

    // Initialize and run database migrations
    let mut migration_manager = DatabaseMigrationManager::new(db_pool.pool().clone()).await?;

    // Register built-in migrations
    register_core_migrations(&mut migration_manager);

    // Validate schema before applying migrations
    let validation_report = migration_manager.validate_schema().await?;
    if !validation_report.is_valid {
        warn!(
            "Schema validation found {} issues",
            validation_report.issues.len()
        );
        for issue in &validation_report.issues {
            warn!("Schema issue: {}", issue);
        }
    }

    // Apply pending migrations
    match migration_manager.apply_migrations().await {
        Ok(applied) => {
            if applied.is_empty() {
                info!("Database schema is up to date");
            } else {
                info!("Successfully applied {} database migrations", applied.len());
            }
        }
        Err(e) => {
            error!("Failed to apply migrations: {}", e);
            return Err(e);
        }
    }

    // Run performance benchmarks to ensure queries meet <2s requirement
    info!("Running database performance benchmarks...");
    match QueryPerformanceMonitor::benchmark_queries(db_pool.pool()).await {
        Ok(results) => {
            let slow_queries: Vec<_> = results
                .iter()
                .filter(|r| r.execution_time_ms > 2000)
                .collect();

            if slow_queries.is_empty() {
                info!("All queries performed within 2s threshold");
            } else {
                for slow_query in &slow_queries {
                    warn!(
                        "Query '{}' exceeded 2s threshold: {}ms",
                        slow_query.query_name, slow_query.execution_time_ms
                    );
                }
                warn!(
                    "Found {} queries exceeding 2s performance threshold",
                    slow_queries.len()
                );
            }
        }
        Err(e) => {
            warn!("Performance benchmark failed: {}", e);
            // Don't fail startup for benchmark failures
        }
    }

    // Initialize MCP server
    let mcp_server = McpServer::new(db_pool).await?;

    // Start HTTP server with graceful shutdown
    let addr = format!("0.0.0.0:{port}");
    info!("Doc Server listening on {}", addr);

    run_server_with_graceful_shutdown(mcp_server, &addr).await?;

    Ok(())
}

/// Run the server with graceful shutdown signal handling
async fn run_server_with_graceful_shutdown(mcp_server: McpServer, addr: &str) -> Result<()> {
    use tokio::net::TcpListener;

    // Create router and bind listener
    let app = mcp_server.create_router();
    let listener = TcpListener::bind(addr).await?;
    info!(
        "Server listening on {} with graceful shutdown enabled",
        addr
    );

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Wait for shutdown signal (SIGTERM or SIGINT)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            warn!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
        },
        () = terminate => {
            warn!("Received SIGTERM, initiating graceful shutdown...");
        }
    }

    info!("Shutdown signal received, starting graceful shutdown (timeout: 30s)");
}

/// Register core database migrations
#[allow(clippy::too_many_lines)]
fn register_core_migrations(migration_manager: &mut DatabaseMigrationManager) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Helper function to calculate checksums
    fn calculate_checksum(sql: &str) -> String {
        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    // Migration 1: Core extensions
    let extensions_sql = r#"
        CREATE EXTENSION IF NOT EXISTS vector;
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
    "#;
    migration_manager.register_migration(MigrationInfo {
        id: "001_core_extensions".to_string(),
        version: "1.0.0".to_string(),
        description: "Install required PostgreSQL extensions (vector, uuid-ossp)".to_string(),
        up_sql: extensions_sql.to_string(),
        down_sql: Some(
            r#"DROP EXTENSION IF EXISTS vector; DROP EXTENSION IF EXISTS "uuid-ossp";"#.to_string(),
        ),
        dependencies: vec![],
        checksum: calculate_checksum(extensions_sql),
    });

    // Migration 2: Create enum types
    let enum_sql = r"
        DO $$ BEGIN
            CREATE TYPE doc_type AS ENUM (
                'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
                'meteora', 'raydium', 'solana', 'ebpf', 'rust_best_practices'
            );
        EXCEPTION
            WHEN duplicate_object THEN null;
        END $$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "002_enum_types".to_string(),
        version: "1.0.0".to_string(),
        description: "Create doc_type enum for document categorization".to_string(),
        up_sql: enum_sql.to_string(),
        down_sql: Some("DROP TYPE IF EXISTS doc_type;".to_string()),
        dependencies: vec!["001_core_extensions".to_string()],
        checksum: calculate_checksum(enum_sql),
    });

    // Migration 3: Create documents table
    let documents_sql = r"
        CREATE TABLE IF NOT EXISTS documents (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            doc_type doc_type NOT NULL,
            source_name VARCHAR(255) NOT NULL,
            doc_path TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata JSONB DEFAULT '{}',
            embedding vector(3072),
            token_count INTEGER,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(doc_type, source_name, doc_path)
        );
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "003_documents_table".to_string(),
        version: "1.0.0".to_string(),
        description: "Create documents table for storing documentation content and embeddings"
            .to_string(),
        up_sql: documents_sql.to_string(),
        down_sql: Some("DROP TABLE IF EXISTS documents;".to_string()),
        dependencies: vec!["002_enum_types".to_string()],
        checksum: calculate_checksum(documents_sql),
    });

    // Migration 4: Create document_sources table
    let sources_sql = r"
        CREATE TABLE IF NOT EXISTS document_sources (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            doc_type doc_type NOT NULL,
            source_name VARCHAR(255) NOT NULL,
            config JSONB DEFAULT '{}',
            enabled BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(doc_type, source_name)
        );
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "004_document_sources_table".to_string(),
        version: "1.0.0".to_string(),
        description: "Create document_sources table for source configuration management"
            .to_string(),
        up_sql: sources_sql.to_string(),
        down_sql: Some("DROP TABLE IF EXISTS document_sources;".to_string()),
        dependencies: vec!["002_enum_types".to_string()],
        checksum: calculate_checksum(sources_sql),
    });

    // Migration 5: Create indexes for performance
    let indexes_sql = r"
        CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
        CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
        CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_document_sources_doc_type ON document_sources(doc_type);
        CREATE INDEX IF NOT EXISTS idx_document_sources_enabled ON document_sources(enabled);
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "005_core_indexes".to_string(),
        version: "1.0.0".to_string(),
        description: "Create performance indexes for documents and document_sources tables"
            .to_string(),
        up_sql: indexes_sql.to_string(),
        down_sql: Some(
            r"
            DROP INDEX IF EXISTS idx_documents_doc_type;
            DROP INDEX IF EXISTS idx_documents_source_name;
            DROP INDEX IF EXISTS idx_documents_created_at;
            DROP INDEX IF EXISTS idx_document_sources_doc_type;
            DROP INDEX IF EXISTS idx_document_sources_enabled;
        "
            .to_string(),
        ),
        dependencies: vec![
            "003_documents_table".to_string(),
            "004_document_sources_table".to_string(),
        ],
        checksum: calculate_checksum(indexes_sql),
    });

    // Migration 6: Add foreign key constraint
    let fk_sql = r"
        ALTER TABLE documents 
        ADD CONSTRAINT IF NOT EXISTS fk_documents_source 
        FOREIGN KEY (doc_type, source_name) 
        REFERENCES document_sources(doc_type, source_name);
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "006_foreign_keys".to_string(),
        version: "1.1.0".to_string(),
        description: "Add foreign key constraints between documents and document_sources"
            .to_string(),
        up_sql: fk_sql.to_string(),
        down_sql: Some(
            "ALTER TABLE documents DROP CONSTRAINT IF EXISTS fk_documents_source;".to_string(),
        ),
        dependencies: vec![
            "003_documents_table".to_string(),
            "004_document_sources_table".to_string(),
        ],
        checksum: calculate_checksum(fk_sql),
    });

    // Migration 7: Add partitioning for documents table (range partitioning by created_at)
    let partitioning_sql = r"
        -- Convert documents table to partitioned table (by monthly ranges)
        -- Dynamically create partitions to cover existing data and a small future window
        DO $$
        DECLARE
            start_month DATE;
            end_month DATE;
            current_month DATE;
            partition_name TEXT;
        BEGIN
            -- Create the partitioned parent table
            CREATE TABLE IF NOT EXISTS documents_partitioned (
                LIKE documents INCLUDING ALL
            ) PARTITION BY RANGE (created_at);

            -- Create a DEFAULT partition to avoid insert failures for out-of-range data
            IF to_regclass('public.documents_default') IS NULL THEN
                EXECUTE 'CREATE TABLE documents_default PARTITION OF documents_partitioned DEFAULT';
            END IF;

            -- Determine the range of months to create
            SELECT date_trunc('month', COALESCE(MIN(created_at), CURRENT_DATE))::date INTO start_month
            FROM documents;

            SELECT GREATEST(
                date_trunc('month', CURRENT_DATE + INTERVAL '3 months')::date,
                COALESCE(date_trunc('month', MAX(created_at))::date, CURRENT_DATE::date)
            ) INTO end_month
            FROM documents;

            current_month := start_month;
            WHILE current_month <= end_month LOOP
                partition_name := format('documents_y%sm%s', to_char(current_month, 'YYYY'), to_char(current_month, 'MM'));
                -- Create monthly partition if missing
                IF to_regclass(partition_name) IS NULL THEN
                    EXECUTE format(
                        'CREATE TABLE %I PARTITION OF documents_partitioned FOR VALUES FROM (%L) TO (%L)',
                        partition_name,
                        current_month::timestamptz,
                        (current_month + INTERVAL '1 month')::timestamptz
                    );
                END IF;

                -- Copy data for this month in a bounded batch
                EXECUTE format(
                    'INSERT INTO documents_partitioned SELECT * FROM documents WHERE created_at >= %L AND created_at < %L',
                    current_month::timestamptz,
                    (current_month + INTERVAL '1 month')::timestamptz
                );

                current_month := (current_month + INTERVAL '1 month')::date;
            END LOOP;

            -- Copy any rows with NULL created_at into default partition
            EXECUTE 'INSERT INTO documents_partitioned SELECT * FROM documents WHERE created_at IS NULL';

            -- Swap tables atomically after data is copied
            ALTER TABLE documents RENAME TO documents_old;
            ALTER TABLE documents_partitioned RENAME TO documents;
        END
        $$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "007_partitioning".to_string(),
        version: "1.2.0".to_string(),
        description:
            "Add monthly range partitioning to documents table for performance and archival"
                .to_string(),
        up_sql: partitioning_sql.to_string(),
        down_sql: Some(
            r"
            -- Restore non-partitioned table (data loss risk!)
            CREATE TABLE documents_temp (LIKE documents_old INCLUDING ALL);
            INSERT INTO documents_temp SELECT * FROM documents;
            DROP TABLE documents;
            ALTER TABLE documents_temp RENAME TO documents;
        "
            .to_string(),
        ),
        dependencies: vec!["006_foreign_keys".to_string()],
        checksum: calculate_checksum(partitioning_sql),
    });

    // Migration 8: Create archival policies and procedures
    let archival_sql = r"
        -- Create archived_documents table for long-term storage
        CREATE TABLE IF NOT EXISTS archived_documents (
            LIKE documents INCLUDING ALL
        );
        
        -- Add archival metadata
        ALTER TABLE archived_documents 
        ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
        ADD COLUMN IF NOT EXISTS archival_reason TEXT;
        
        -- Create function to archive old documents (>1 year old)
        CREATE OR REPLACE FUNCTION archive_old_documents() RETURNS INTEGER
        LANGUAGE plpgsql AS $$
        DECLARE
            archived_count INTEGER := 0;
        BEGIN
            -- Move documents older than 1 year to archive
            WITH archived_rows AS (
                DELETE FROM documents 
                WHERE created_at < CURRENT_DATE - INTERVAL '1 year'
                RETURNING *
            )
            INSERT INTO archived_documents (
                id, doc_type, source_name, doc_path, content, metadata, 
                embedding, token_count, created_at, updated_at,
                archived_at, archival_reason
            )
            SELECT 
                id, doc_type, source_name, doc_path, content, metadata,
                embedding, token_count, created_at, updated_at,
                CURRENT_TIMESTAMP, 'Automatic archival - age > 1 year'
            FROM archived_rows;
            
            GET DIAGNOSTICS archived_count = ROW_COUNT;
            
            -- Log archival operation
            INSERT INTO migration_history (migration_id, version, status, applied_by, execution_time_ms)
            VALUES (
                'archival_' || to_char(CURRENT_TIMESTAMP, 'YYYY_MM_DD_HH24_MI_SS'),
                '1.2.0',
                'completed',
                'archival_function',
                0
            );
            
            RETURN archived_count;
        END;
        $$;
        
        -- Create indexes on archived table
        CREATE INDEX IF NOT EXISTS idx_archived_documents_created_at ON archived_documents(created_at);
        CREATE INDEX IF NOT EXISTS idx_archived_documents_archived_at ON archived_documents(archived_at);
        CREATE INDEX IF NOT EXISTS idx_archived_documents_doc_type ON archived_documents(doc_type);
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "008_archival_policy".to_string(),
        version: "1.2.0".to_string(),
        description: "Create archival system for old documents (>1 year) with automated procedures"
            .to_string(),
        up_sql: archival_sql.to_string(),
        down_sql: Some(
            r"
            DROP FUNCTION IF EXISTS archive_old_documents();
            DROP TABLE IF EXISTS archived_documents;
        "
            .to_string(),
        ),
        dependencies: vec!["007_partitioning".to_string()],
        checksum: calculate_checksum(archival_sql),
    });
}

/// Run database migrations only (for K8s migration jobs)
async fn run_migrations_only() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,doc_server=debug".to_string()),
        )
        .init();

    info!("Running database schema migrations...");

    // Get database configuration
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Initialize database connection
    let db_pool = DatabasePool::new(&database_url).await?;

    // Initialize migration manager
    let mut migration_manager = DatabaseMigrationManager::new(db_pool.pool().clone()).await?;

    // Register core migrations
    register_core_migrations(&mut migration_manager);

    // Validate schema before applying migrations
    info!("Validating current database schema...");
    let validation_report = migration_manager.validate_schema().await?;
    if !validation_report.is_valid {
        warn!(
            "Schema validation found {} issues",
            validation_report.issues.len()
        );
        for issue in &validation_report.issues {
            warn!("Schema issue: {}", issue);
        }
    }

    // Apply pending migrations
    info!("Applying pending database migrations...");
    match migration_manager.apply_migrations().await {
        Ok(applied) => {
            if applied.is_empty() {
                info!("Database schema is already up to date");
            } else {
                info!("Successfully applied {} database migrations", applied.len());
                for migration in &applied {
                    info!(
                        "Applied migration: {} ({}ms)",
                        migration.migration_id, migration.execution_time_ms
                    );
                }
            }
        }
        Err(e) => {
            error!("Failed to apply migrations: {}", e);
            return Err(e);
        }
    }

    // Final validation
    info!("Running post-migration validation...");
    let final_report = migration_manager.validate_schema().await?;
    if final_report.is_valid {
        info!("Schema validation passed - database ready for operation");
    } else {
        error!(
            "Post-migration validation failed with {} issues",
            final_report.issues.len()
        );
        for issue in &final_report.issues {
            error!("Validation issue: {}", issue);
        }
        return Err(anyhow::anyhow!("Post-migration validation failed"));
    }

    // Get migration status summary
    let status = migration_manager.get_migration_status().await?;
    info!(
        "Migration status: {} completed, {} failed, {} pending",
        status.completed, status.failed, status.pending
    );

    info!("Database migration completed successfully");
    Ok(())
}
