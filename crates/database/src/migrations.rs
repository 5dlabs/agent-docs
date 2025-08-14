//! Database migrations and schema management

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

/// Database migration operations
pub struct Migrations;

impl Migrations {
    /// Run all pending migrations
    ///
    /// # Errors
    ///
    /// Returns an error if executing any migration SQL statement fails or the
    /// database connection encounters an error during execution.
    pub async fn run(pool: &PgPool) -> Result<()> {
        info!("Running database migrations...");

        // Create extensions
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(pool)
            .await?;

        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"")
            .execute(pool)
            .await?;

        // Create enum types
        sqlx::query(
            r"
            DO $$ BEGIN
                CREATE TYPE doc_type AS ENUM (
                    'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
                    'meteora', 'raydium', 'solana', 'ebpf', 'rust_best_practices'
                );
            EXCEPTION
                WHEN duplicate_object THEN null;
            END $$;
        ",
        )
        .execute(pool)
        .await?;

        // Create documents table
        sqlx::query(
            r"
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
            )
        ",
        )
        .execute(pool)
        .await?;

        // Create document_sources table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS document_sources (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                doc_type doc_type NOT NULL,
                source_name VARCHAR(255) NOT NULL,
                config JSONB DEFAULT '{}',
                enabled BOOLEAN DEFAULT true,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(doc_type, source_name)
            )
        ",
        )
        .execute(pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type)")
            .execute(pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name)",
        )
        .execute(pool)
        .await?;

        // Note: Skipping vector index for 3072-dimensional embeddings due to pgvector 2000-dimension limit
        // Queries will still work but be slower. Consider upgrading pgvector or using 1536 dimensions if performance is critical.

        info!("Database migrations completed successfully");
        Ok(())
    }
}
