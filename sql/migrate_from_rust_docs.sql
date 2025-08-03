-- Migration script from rust_docs_vectors to new docs database
-- This script assumes you're connected to the NEW docs database
-- and have access to the old rust_docs_vectors database

-- Step 1: Migrate crate information to document_sources
INSERT INTO document_sources (
    doc_type, 
    source_name, 
    version, 
    config, 
    enabled, 
    last_updated, 
    total_docs, 
    total_tokens,
    created_at,
    updated_at
)
SELECT 
    'rust'::doc_type as doc_type,
    name as source_name,
    version,
    jsonb_build_object(
        'docs_rs_url', 'https://docs.rs/' || name || '/' || COALESCE(version, 'latest'),
        'migrated_from', 'rust_docs_vectors'
    ) as config,
    true as enabled,
    last_updated,
    total_docs,
    total_tokens,
    COALESCE(last_updated, CURRENT_TIMESTAMP) as created_at,
    CURRENT_TIMESTAMP as updated_at
FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT name, version, last_updated, total_docs, total_tokens FROM crates'
) AS old_crates(
    name VARCHAR(255),
    version VARCHAR(50),
    last_updated TIMESTAMP,
    total_docs INTEGER,
    total_tokens INTEGER
);

-- Step 2: Migrate document embeddings to documents table
INSERT INTO documents (
    doc_type,
    source_name,
    doc_path,
    content,
    metadata,
    embedding,
    token_count,
    created_at,
    updated_at
)
SELECT 
    'rust'::doc_type as doc_type,
    crate_name as source_name,
    doc_path,
    content,
    jsonb_build_object(
        'crate_name', crate_name,
        'migrated_from', 'rust_docs_vectors',
        'original_id', id
    ) as metadata,
    embedding,
    token_count,
    COALESCE(created_at, CURRENT_TIMESTAMP) as created_at,
    CURRENT_TIMESTAMP as updated_at
FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT id, crate_name, doc_path, content, embedding, token_count, created_at FROM doc_embeddings'
) AS old_docs(
    id INTEGER,
    crate_name VARCHAR(255),
    doc_path TEXT,
    content TEXT,
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMP
);

-- Step 3: Update document_sources statistics based on actual migrated data
UPDATE document_sources 
SET 
    total_docs = (
        SELECT COUNT(*) 
        FROM documents 
        WHERE documents.source_name = document_sources.source_name 
        AND documents.doc_type = document_sources.doc_type
    ),
    total_tokens = (
        SELECT COALESCE(SUM(token_count), 0) 
        FROM documents 
        WHERE documents.source_name = document_sources.source_name 
        AND documents.doc_type = document_sources.doc_type
    )
WHERE doc_type = 'rust';

-- Step 4: Verification queries (run these manually to verify migration)
/*
-- Verify crate count
SELECT 'document_sources' as table_name, COUNT(*) as count FROM document_sources WHERE doc_type = 'rust'
UNION ALL
SELECT 'original_crates' as table_name, COUNT(*) as count FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT COUNT(*) FROM crates'
) AS count_result(count BIGINT);

-- Verify document count  
SELECT 'documents' as table_name, COUNT(*) as count FROM documents WHERE doc_type = 'rust'
UNION ALL
SELECT 'original_doc_embeddings' as table_name, COUNT(*) as count FROM dblink(
    'host=localhost dbname=rust_docs_vectors user=' || current_user,
    'SELECT COUNT(*) FROM doc_embeddings'
) AS count_result(count BIGINT);

-- Verify sample data
SELECT source_name, COUNT(*) as doc_count, SUM(token_count) as total_tokens
FROM documents 
WHERE doc_type = 'rust' 
GROUP BY source_name 
ORDER BY doc_count DESC 
LIMIT 10;

-- Test vector search still works
SELECT source_name, doc_path, content
FROM documents 
WHERE doc_type = 'rust' 
AND embedding IS NOT NULL
ORDER BY embedding <-> (SELECT embedding FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL LIMIT 1)
LIMIT 5;
*/