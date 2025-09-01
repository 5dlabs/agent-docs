-- Test Database Setup Script
-- This script sets up the required tables and schema for CI testing
-- Run this before running tests in CI environment

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";

-- Create doc_type enum if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'doc_type') THEN
        CREATE TYPE doc_type AS ENUM (
            'rust',
            'jupyter',
            'birdeye',
            'cilium',
            'talos',
            'meteora',
            'raydium',
            'solana',
            'ebpf',
            'rust_best_practices'
        );
    END IF;
END $$;

-- Create document_sources table
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

-- Create documents table
DO $$
BEGIN
    CREATE TABLE IF NOT EXISTS documents (
        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
        doc_type doc_type NOT NULL,
        source_name VARCHAR(255) NOT NULL,
        doc_path TEXT NOT NULL,
        content TEXT NOT NULL,
        metadata JSONB DEFAULT '{}',
        -- Use vector type if available, otherwise use a placeholder
        embedding TEXT,
        token_count INTEGER,
        created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
        UNIQUE(doc_type, source_name, doc_path),
        FOREIGN KEY (doc_type, source_name) REFERENCES document_sources(doc_type, source_name) ON DELETE CASCADE
    );

    -- Try to convert embedding column to vector type if extension is available
    BEGIN
        -- Check if we have ALTER permission on the documents table
        IF EXISTS (
            SELECT 1 FROM information_schema.table_privileges
            WHERE table_name = 'documents'
            AND table_schema = 'public'
            AND privilege_type = 'ALTER'
            AND grantee = current_user
        ) THEN
            ALTER TABLE documents ALTER COLUMN embedding TYPE vector(3072);
            RAISE NOTICE 'Converted embedding column to vector type';
        ELSE
            RAISE NOTICE 'No ALTER permission on documents table, skipping vector conversion';
        END IF;
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Vector extension not available, keeping TEXT type for embeddings';
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Insufficient privileges to alter documents table';
    END;
END $$;

-- Create indexes for performance (only if we have permissions)
DO $$
BEGIN
    -- Try to create indexes, handle permission errors gracefully
    BEGIN
        CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
        CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
        CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);
        RAISE NOTICE 'Created performance indexes on documents table';
    EXCEPTION
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Insufficient privileges to create indexes on documents table';
    END;
END;
$$;

-- Insert some test data into document_sources to satisfy foreign key constraints
-- Insert a broader set of test sources for different doc types (conditional on permissions)
DO $$
BEGIN
    -- Check if we have INSERT permission and if the constraint exists
    IF EXISTS (
        SELECT 1 FROM information_schema.table_privileges
        WHERE table_name = 'document_sources'
        AND table_schema = 'public'
        AND privilege_type = 'INSERT'
        AND grantee = current_user
    ) THEN
        -- Try INSERT with ON CONFLICT first (if constraint exists)
        BEGIN
            INSERT INTO document_sources (doc_type, source_name, config, enabled)
            VALUES
                ('rust', 'test_crate', '{"version": "1.0.0"}', true),
                ('rust', 'test_crate_2', '{"version": "2.0.0"}', true),
                ('rust', 'db-test-crate-', '{"version": "0.1.0"}', true),
                ('jupyter', 'test_notebook', '{"kernel": "python3"}', true),
                ('birdeye', 'test_birdeye', '{"api_version": "v1"}', true),
                ('solana', 'test_solana', '{"network": "mainnet"}', true)
            ON CONFLICT (doc_type, source_name) DO NOTHING;
            RAISE NOTICE 'Inserted test data into document_sources with ON CONFLICT';
        EXCEPTION
            WHEN undefined_object THEN
                -- Constraint doesn't exist, try INSERT without ON CONFLICT
                INSERT INTO document_sources (doc_type, source_name, config, enabled)
                VALUES
                    ('rust', 'test_crate', '{"version": "1.0.0"}', true),
                    ('rust', 'test_crate_2', '{"version": "2.0.0"}', true),
                    ('rust', 'db-test-crate-', '{"version": "0.1.0"}', true),
                    ('jupyter', 'test_notebook', '{"kernel": "python3"}', true),
                    ('birdeye', 'test_birdeye', '{"api_version": "v1"}', true),
                    ('solana', 'test_solana', '{"network": "mainnet"}', true)
                ON CONFLICT DO NOTHING; -- Generic conflict handling
                RAISE NOTICE 'Inserted test data into document_sources with generic ON CONFLICT';
            WHEN unique_violation THEN
                -- Handle unique violations gracefully
                RAISE NOTICE 'Some test data already exists in document_sources, skipping inserts';
        END;
    ELSE
        RAISE NOTICE 'No INSERT permission on document_sources table, skipping test data insertion';
    END IF;
EXCEPTION
    WHEN insufficient_privilege THEN
        RAISE NOTICE 'Insufficient privileges to insert into document_sources table';
END;
$$;

-- Create a function to automatically insert document_sources for test data
-- Handle permission errors gracefully
DO $$
BEGIN
    -- Try to create the function and trigger
    BEGIN
        CREATE OR REPLACE FUNCTION ensure_document_source_exists()
        RETURNS TRIGGER AS $$
        BEGIN
            -- Only auto-insert for test-like source names (containing 'test' or 'db-test')
            IF NEW.source_name LIKE '%test%' OR NEW.source_name LIKE 'db-test%' THEN
                -- Try to insert, handle all constraint issues
                BEGIN
                    INSERT INTO document_sources (doc_type, source_name, config, enabled)
                    VALUES (NEW.doc_type, NEW.source_name, '{"auto_created": true}', true);
                EXCEPTION
                    WHEN unique_violation THEN
                        -- Record already exists, do nothing
                        NULL;
                END;
            END IF;

            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;

        -- Create trigger to auto-insert document sources for test data
        DROP TRIGGER IF EXISTS ensure_document_source_trigger ON documents;
        CREATE TRIGGER ensure_document_source_trigger
            BEFORE INSERT ON documents
            FOR EACH ROW EXECUTE FUNCTION ensure_document_source_exists();

        RAISE NOTICE 'Created function and trigger for document source auto-insertion';
    EXCEPTION
        WHEN insufficient_privilege THEN
            RAISE NOTICE 'Insufficient privileges to create function and trigger';
    END;
END;
$$;

-- Create job_status enum if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'job_status') THEN
        CREATE TYPE job_status AS ENUM ('queued', 'running', 'completed', 'failed', 'cancelled');
    END IF;
END $$;

-- Create crate_jobs table for background job tracking
CREATE TABLE IF NOT EXISTS crate_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    crate_name VARCHAR(255) NOT NULL,
    operation VARCHAR(50) NOT NULL CHECK (operation IN ('add_crate', 'remove_crate')),
    status job_status DEFAULT 'queued',
    progress INTEGER CHECK (progress >= 0 AND progress <= 100),
    error TEXT,
    started_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Add indexes for performance
CREATE INDEX IF NOT EXISTS idx_crate_jobs_crate_name ON crate_jobs(crate_name);
CREATE INDEX IF NOT EXISTS idx_crate_jobs_status ON crate_jobs(status);
CREATE INDEX IF NOT EXISTS idx_crate_jobs_operation ON crate_jobs(operation);
CREATE INDEX IF NOT EXISTS idx_crate_jobs_started_at ON crate_jobs(started_at DESC);

-- Create function to update updated_at if it doesn't exist
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to update updated_at columns
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'update_crate_jobs_updated_at') THEN
        CREATE TRIGGER update_crate_jobs_updated_at
            BEFORE UPDATE ON crate_jobs
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'update_documents_updated_at') THEN
        CREATE TRIGGER update_documents_updated_at
            BEFORE UPDATE ON documents
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'update_document_sources_updated_at') THEN
        CREATE TRIGGER update_document_sources_updated_at
            BEFORE UPDATE ON document_sources
            FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    END IF;
END $$;

-- Create function to clean up old completed jobs (older than 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_crate_jobs()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM crate_jobs
    WHERE status IN ('completed', 'failed', 'cancelled')
      AND finished_at < CURRENT_TIMESTAMP - INTERVAL '30 days';

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Create migration_history table if it doesn't exist
CREATE TABLE IF NOT EXISTS migration_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    migration_id VARCHAR(255) UNIQUE NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT,
    applied_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    checksum VARCHAR(64),
    up_sql TEXT NOT NULL,
    down_sql TEXT,
    dependencies TEXT[] DEFAULT ARRAY[]::TEXT[]
);

-- Create index on migration_history
CREATE INDEX IF NOT EXISTS idx_migration_history_migration_id ON migration_history(migration_id);

COMMIT;
