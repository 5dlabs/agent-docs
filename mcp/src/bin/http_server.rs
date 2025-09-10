//! HTTP server binary for the Doc Server
//!
//! This binary provides the main HTTP endpoint for MCP communication with Streamable HTTP transport support.

use anyhow::Result;
use db::{DatabaseMigrationManager, DatabasePool, MigrationInfo, QueryPerformanceMonitor};
use dotenvy::dotenv;
use mcp::McpServer;
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
    // Prefer PORT, then MCP_PORT, default to 3001 to align with container defaults
    let port = env::var("PORT")
        .or_else(|_| env::var("MCP_PORT"))
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .expect("PORT/MCP_PORT must be a valid number");

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
    // Allow host override via MCP_HOST; default to all interfaces
    let host = env::var("MCP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{host}:{port}");
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
        -- Vector extension may not be available in test environments
        DO $$
        BEGIN
            BEGIN
                CREATE EXTENSION IF NOT EXISTS vector;
            EXCEPTION
                WHEN insufficient_privilege THEN
                    RAISE NOTICE 'Vector extension not available, skipping';
            END;

            CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
        END $$;
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

    // Migration 2: Create doc_type as TEXT (dynamic types)
    let enum_sql = r"
    -- With dynamic DocType as String, we use TEXT instead of enum
    -- This allows any doc_type value from tools.json to be stored
    DO $$ BEGIN
        -- Create doc_type as TEXT if it doesn't exist as enum
        IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'doc_type') THEN
            -- We'll handle this as TEXT in the table definition
            NULL;
        END IF;
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
        DO $$
        BEGIN
            -- Drop existing enum if it exists and recreate as TEXT
            DO $$
            BEGIN
                -- Check if doc_type column exists as enum and convert to TEXT
                IF EXISTS (
                    SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'documents'
                    AND column_name = 'doc_type'
                    AND data_type = 'USER-DEFINED'
                ) THEN
                    ALTER TABLE documents ALTER COLUMN doc_type TYPE TEXT;
                    RAISE NOTICE 'Converted existing doc_type enum column to TEXT';
                END IF;
            EXCEPTION
                WHEN undefined_table THEN
                    -- Table doesn't exist yet, that's fine
                    NULL;
            END $$;

            CREATE TABLE IF NOT EXISTS documents (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                doc_type TEXT NOT NULL,
                source_name VARCHAR(255) NOT NULL,
                doc_path TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata JSONB DEFAULT '{}',
                -- Use vector type if available, otherwise use a placeholder
                embedding TEXT,
                token_count INTEGER,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(doc_type, source_name, doc_path)
            );

            -- Try to convert embedding column to vector type if extension is available
            BEGIN
                ALTER TABLE documents ALTER COLUMN embedding TYPE vector(3072);
            EXCEPTION
                WHEN undefined_object THEN
                    RAISE NOTICE 'Vector extension not available, keeping TEXT type for embeddings';
            END;
        END $$;
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
            doc_type TEXT NOT NULL,
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
        DO $$
        BEGIN
            CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
            CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
            CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_document_sources_doc_type ON document_sources(doc_type);
            CREATE INDEX IF NOT EXISTS idx_document_sources_enabled ON document_sources(enabled);
        END $$;
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
        DO $$
        BEGIN
            ALTER TABLE documents
            ADD CONSTRAINT fk_documents_source
            FOREIGN KEY (doc_type, source_name)
            REFERENCES document_sources(doc_type, source_name);
        EXCEPTION
            WHEN duplicate_object THEN
                RAISE NOTICE 'Foreign key constraint already exists, skipping';
        END $$;
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
    // NOTE: Disabled for CI/testing environments - too complex for basic schema setup
    let _ = r"
        -- Partitioning disabled for CI/testing - requires unique constraints to include partitioning columns
        DO $$ BEGIN
            RAISE NOTICE 'Partitioning migration skipped in CI/testing environment';
        END $$;
    ";

    /*
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
        checksum: calculate_checksum(_partitioning_sql),
    });
    */

    // Migration 8: Create archival policies and procedures
    let _ = r"
        -- Archival system disabled for CI/testing - creates complex functions that aren't needed for basic testing
        DO $$ BEGIN
            RAISE NOTICE 'Archival migration skipped in CI/testing environment';
        END $$;
    ";

    /*
    let archival_sql = r"
        DO $$
        BEGIN
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
        END $$;
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
        checksum: calculate_checksum(_archival_sql),
    });
    */

    // Migration 9: Create job_status enum (idempotent)
    let job_status_sql = r"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'job_status') THEN
                CREATE TYPE job_status AS ENUM ('queued', 'running', 'completed', 'failed', 'cancelled');
            END IF;
        END $$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "009_job_status_enum".to_string(),
        version: "1.0.0".to_string(),
        description: "Create job_status enum for background job tracking".to_string(),
        up_sql: job_status_sql.to_string(),
        down_sql: Some("DROP TYPE IF EXISTS job_status;".to_string()),
        dependencies: vec!["003_documents_table".to_string()],
        checksum: calculate_checksum(job_status_sql),
    });

    // Migration 10: Create ingest_jobs table (idempotent)
    let ingest_jobs_sql = r"
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'ingest_jobs'
            ) THEN
                CREATE TABLE ingest_jobs (
                    id UUID PRIMARY KEY,
                    url TEXT NOT NULL,
                    doc_type TEXT NOT NULL,
                    status job_status DEFAULT 'queued',
                    started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    finished_at TIMESTAMPTZ NULL,
                    output TEXT NULL,
                    error TEXT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
                );

                CREATE INDEX idx_ingest_jobs_status ON ingest_jobs(status);
                CREATE INDEX idx_ingest_jobs_started_at ON ingest_jobs(started_at DESC);
            END IF;
        END $$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "010_ingest_jobs_table".to_string(),
        version: "1.0.0".to_string(),
        description: "Create ingest_jobs table for intelligent ingestion tracking".to_string(),
        up_sql: ingest_jobs_sql.to_string(),
        down_sql: Some(
            r"
            DROP INDEX IF EXISTS idx_ingest_jobs_status;
            DROP INDEX IF EXISTS idx_ingest_jobs_started_at;
            DROP TABLE IF EXISTS ingest_jobs;
            "
            .to_string(),
        ),
        dependencies: vec!["009_job_status_enum".to_string()],
        checksum: calculate_checksum(ingest_jobs_sql),
    });

    // Migration 11: Create search-related indexes (optional, idempotent)
    let search_indexes_sql = r"
        DO $$
        BEGIN
            -- Optional extension for trigram index; skip if not permitted
            BEGIN
                CREATE EXTENSION IF NOT EXISTS pg_trgm;
            EXCEPTION
                WHEN insufficient_privilege THEN
                    RAISE NOTICE 'pg_trgm extension not available, skipping';
            END;

            -- GIN index on FTS expression to accelerate websearch_to_tsquery filtering
            CREATE INDEX IF NOT EXISTS idx_documents_fts
            ON documents USING GIN (to_tsvector('english', coalesce(content,'')));

            -- Trigram index for doc_path fuzzy matches
            CREATE INDEX IF NOT EXISTS idx_documents_doc_path_trgm
            ON documents USING GIN (doc_path gin_trgm_ops);
        END $$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "011_search_indexes".to_string(),
        version: "1.2.0".to_string(),
        description: "Add FTS and trigram indexes to improve query latency".to_string(),
        up_sql: search_indexes_sql.to_string(),
        down_sql: Some(
            r"
            DROP INDEX IF EXISTS idx_documents_fts;
            DROP INDEX IF EXISTS idx_documents_doc_path_trgm;
        "
            .to_string(),
        ),
        dependencies: vec!["003_documents_table".to_string()],
        checksum: calculate_checksum(search_indexes_sql),
    });

    // Migration 12: Force doc_type columns to TEXT and drop legacy enum/checks
    let force_text_sql = r"
        DO $$
        DECLARE
            tbl text;
            con record;
        BEGIN
            -- Convert doc_type columns to TEXT if they are enum/user-defined
            FOREACH tbl IN ARRAY ARRAY['documents','document_sources','archived_documents'] LOOP
                IF EXISTS (
                    SELECT 1 FROM information_schema.columns
                    WHERE table_schema='public' AND table_name=tbl AND column_name='doc_type' AND data_type='USER-DEFINED'
                ) THEN
                    EXECUTE format('ALTER TABLE %I ALTER COLUMN doc_type TYPE TEXT USING doc_type::text', tbl);
                    RAISE NOTICE 'Converted %.doc_type to TEXT', tbl;
                END IF;

                -- Drop any CHECK constraints that reference doc_type
                FOR con IN
                    SELECT conname, pg_get_constraintdef(oid) AS def
                    FROM pg_constraint
                    WHERE conrelid = to_regclass('public.'||tbl)
                      AND contype = 'c'
                LOOP
                    IF position('doc_type' in con.def) > 0 THEN
                        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT %I', tbl, con.conname);
                        RAISE NOTICE 'Dropped constraint %.% on %', con.conname, con.def, tbl;
                    END IF;
                END LOOP;
            END LOOP;

            -- Drop legacy enum type if it still exists
            IF EXISTS (SELECT 1 FROM pg_type WHERE typname='doc_type') THEN
                BEGIN
                    EXECUTE 'DROP TYPE doc_type';
                    RAISE NOTICE 'Dropped legacy enum type doc_type';
                EXCEPTION WHEN dependent_objects_still_exist THEN
                    RAISE NOTICE 'Could not drop enum doc_type due to remaining dependencies';
                END;
            END IF;
        END$$;
    ";
    migration_manager.register_migration(MigrationInfo {
        id: "012_force_doc_type_text".to_string(),
        version: "1.3.0".to_string(),
        description: "Convert doc_type columns to TEXT and drop legacy enum/check constraints"
            .to_string(),
        up_sql: force_text_sql.to_string(),
        down_sql: Some("-- irreversible migration; no-op".to_string()),
        dependencies: vec![
            "003_documents_table".to_string(),
            "004_document_sources_table".to_string(),
        ],
        checksum: calculate_checksum(force_text_sql),
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
