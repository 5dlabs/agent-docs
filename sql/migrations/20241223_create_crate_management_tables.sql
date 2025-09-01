-- Migration: Create Crate Jobs Table Only
-- Date: 2024-12-23
-- Description: Add support for background job tracking for crate operations
-- Note: No separate crates table - using existing documents and document_sources tables

-- Create job_status enum
CREATE TYPE job_status AS ENUM ('queued', 'running', 'completed', 'failed', 'cancelled');

-- Create crate_jobs table for background job tracking
CREATE TABLE crate_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    crate_name VARCHAR(255) NOT NULL,
    operation VARCHAR(50) NOT NULL CHECK (operation IN ('add_crate', 'remove_crate')),
    status job_status DEFAULT 'queued',
    progress INTEGER CHECK (progress >= 0 AND progress <= 100),
    error TEXT,
    started_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Add indexes for performance
CREATE INDEX idx_crate_jobs_crate_name ON crate_jobs(crate_name);
CREATE INDEX idx_crate_jobs_status ON crate_jobs(status);
CREATE INDEX idx_crate_jobs_operation ON crate_jobs(operation);
CREATE INDEX idx_crate_jobs_started_at ON crate_jobs(started_at DESC);

-- Create function to update updated_at if it doesn't exist
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add trigger to update updated_at columns
CREATE TRIGGER update_crate_jobs_updated_at 
    BEFORE UPDATE ON crate_jobs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

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

-- Add comments for documentation
COMMENT ON TABLE crate_jobs IS 'Background jobs for crate management operations with persistent state - job IDs survive restarts';
COMMENT ON FUNCTION cleanup_old_crate_jobs() IS 'Removes old completed/failed jobs to maintain performance';