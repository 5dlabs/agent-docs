-- Doc Server Database Schema
-- Harmonized schema supporting multiple documentation types

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create enum for documentation types
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

-- Main documents table (replaces doc_embeddings)
CREATE TABLE documents (
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

-- Document sources configuration table (replaces crates)
CREATE TABLE document_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type doc_type NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    version VARCHAR(50),
    config JSONB DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_updated TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    total_docs INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Indexes for performance
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_created_at ON documents(created_at);
CREATE INDEX idx_documents_updated_at ON documents(updated_at);

-- Note: pgvector indexes (IVFFlat and HNSW) have a 2000 dimension limit
-- For 3072 dimensions (OpenAI text-embedding-3-large), we skip the index. 
-- Queries will still work but be slower. Consider upgrading pgvector 
-- or using 1536 dimensions if performance is critical.

-- Document sources indexes
CREATE INDEX idx_document_sources_doc_type ON document_sources(doc_type);
CREATE INDEX idx_document_sources_enabled ON document_sources(enabled);
CREATE INDEX idx_document_sources_last_updated ON document_sources(last_updated);

-- Trigger to update updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_document_sources_updated_at 
    BEFORE UPDATE ON document_sources 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Views for easier querying
CREATE VIEW rust_documents AS
SELECT * FROM documents WHERE doc_type = 'rust';

CREATE VIEW active_sources AS
SELECT * FROM document_sources WHERE enabled = true;

-- Function to get document stats by type
CREATE OR REPLACE FUNCTION get_doc_stats(doc_type_param doc_type)
RETURNS TABLE(
    source_name VARCHAR,
    doc_count BIGINT,
    total_tokens BIGINT,
    last_updated TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        ds.source_name,
        COUNT(d.id) as doc_count,
        COALESCE(SUM(d.token_count), 0) as total_tokens,
        MAX(d.updated_at) as last_updated
    FROM document_sources ds
    LEFT JOIN documents d ON ds.source_name = d.source_name AND ds.doc_type = d.doc_type
    WHERE ds.doc_type = doc_type_param
    GROUP BY ds.source_name
    ORDER BY doc_count DESC;
END;
$$ LANGUAGE plpgsql;