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
        ALTER TABLE documents ALTER COLUMN embedding TYPE vector(3072);
    EXCEPTION
        WHEN undefined_object THEN
            RAISE NOTICE 'Vector extension not available, keeping TEXT type for embeddings';
    END;
END $$;

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- Insert some test data into document_sources to satisfy foreign key constraints
-- Insert a broader set of test sources for different doc types
INSERT INTO document_sources (doc_type, source_name, config, enabled)
VALUES
    ('rust', 'test_crate', '{"version": "1.0.0"}', true),
    ('rust', 'test_crate_2', '{"version": "2.0.0"}', true),
    ('rust', 'db-test-crate-', '{"version": "0.1.0"}', true), -- Prefix for UUID-based tests
    ('jupyter', 'test_notebook', '{"kernel": "python3"}', true),
    ('birdeye', 'test_birdeye', '{"api_version": "v1"}', true),
    ('solana', 'test_solana', '{"network": "mainnet"}', true)
ON CONFLICT (doc_type, source_name) DO NOTHING;

-- Insert additional sources with common prefixes that tests might use
INSERT INTO document_sources (doc_type, source_name, config, enabled)
SELECT
    'rust'::doc_type,
    'db-test-crate-' || generate_series(1, 10)::text,
    '{"version": "0.1.0"}'::jsonb,
    true
ON CONFLICT (doc_type, source_name) DO NOTHING;

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
