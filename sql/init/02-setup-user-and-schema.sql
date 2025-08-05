-- Setup user and schema for Doc Server
-- This script runs automatically when PostgreSQL starts in Docker

-- The user 'docserver' should already exist from POSTGRES_USER env var
-- The database 'docs' should already exist from POSTGRES_DB env var

-- Create the documents table with the harmonized schema
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type VARCHAR(50) NOT NULL CHECK (doc_type IN (
        'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
        'meteora', 'solana', 'ebpf', 'raydium', 'rust_best_practices'
    )),
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072), -- OpenAI text-embedding-3-large dimensions
    token_count INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure uniqueness per documentation type
    UNIQUE(doc_type, source_name, doc_path)
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_documents_doc_type ON documents(doc_type);
CREATE INDEX IF NOT EXISTS idx_documents_source_name ON documents(source_name);
CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC);

-- Note: pgvector indexes (IVFFlat and HNSW) have a 2000 dimension limit
-- For 3072 dimensions (OpenAI text-embedding-3-large), we skip the index. 
-- Queries will still work but be slower. Consider upgrading pgvector 
-- or using 1536 dimensions if performance is critical.

-- Create a trigger to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Grant necessary permissions to the docserver user
GRANT ALL PRIVILEGES ON TABLE documents TO docserver;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO docserver;