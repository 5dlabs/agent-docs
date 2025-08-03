# Local Migration Plan

## Overview

This document outlines the local migration steps to transform the current schema to the new harmonized schema, with proper backups before making any changes.

## Current Local State

- **Database**: `rust_docs_vectors` on localhost
- **Content**: 40 Rust crates with 4,133 embeddings
- **Tables**: `crates`, `doc_embeddings`

## Step 1: Create Full Backup

```bash
# Create backup directory
mkdir -p ~/backups/rust_docs_$(date +%Y%m%d)
cd ~/backups/rust_docs_$(date +%Y%m%d)

# Full database backup
pg_dump -h localhost -U jonathonfritz -d rust_docs_vectors -Fc > rust_docs_vectors_full.dump

# Also create SQL format for easy inspection
pg_dump -h localhost -U jonathonfritz -d rust_docs_vectors > rust_docs_vectors_full.sql

# Backup just the data as CSV for safety
psql -h localhost -U jonathonfritz -d rust_docs_vectors << 'EOF'
\COPY crates TO 'crates_backup.csv' WITH CSV HEADER
\COPY doc_embeddings TO 'doc_embeddings_backup.csv' WITH CSV HEADER
EOF

echo "Backup completed at $(pwd)"
```

## Step 2: Test Restoration (Verify Backup)

```bash
# Create test database to verify backup
createdb rust_docs_test
pg_restore -h localhost -U jonathonfritz -d rust_docs_test rust_docs_vectors_full.dump

# Verify counts
psql -h localhost -U jonathonfritz -d rust_docs_test -c "
SELECT 
    (SELECT COUNT(*) FROM crates) as crates_count,
    (SELECT COUNT(*) FROM doc_embeddings) as embeddings_count;"

# Drop test database
dropdb rust_docs_test
```

## Step 3: Create New Schema (In Same Database)

```sql
-- Run this in rust_docs_vectors database
-- This creates new tables alongside existing ones

-- Create new unified tables
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL DEFAULT 'rust',
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name, doc_path)
);

CREATE TABLE document_sources (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL DEFAULT 'rust',
    source_name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_checked TIMESTAMPTZ,
    last_populated TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Create indexes
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_type_source ON documents(doc_type, source_name);
CREATE INDEX idx_documents_metadata ON documents USING gin(metadata);

-- Update trigger function (if doesn't exist)
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers
CREATE TRIGGER update_documents_updated_at 
    BEFORE UPDATE ON documents 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_document_sources_updated_at 
    BEFORE UPDATE ON document_sources 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

## Step 4: Migrate Data to New Schema

```sql
-- Migrate embeddings to documents table
INSERT INTO documents (doc_type, source_name, doc_path, content, embedding, token_count, created_at)
SELECT 
    'rust' as doc_type,
    crate_name as source_name,
    doc_path,
    content,
    embedding,
    token_count,
    created_at
FROM doc_embeddings;

-- Verify migration
SELECT COUNT(*) FROM documents WHERE doc_type = 'rust';
-- Should return 4133

-- Migrate crate configurations
INSERT INTO document_sources (doc_type, source_name, config, enabled, last_populated)
SELECT DISTINCT
    'rust' as doc_type,
    c.name as source_name,
    jsonb_build_object(
        'version', c.version,
        'total_docs', c.total_docs,
        'total_tokens', c.total_tokens
    ) as config,
    true as enabled,
    c.last_updated as last_populated
FROM crates c;

-- Verify source migration
SELECT COUNT(*) FROM document_sources WHERE doc_type = 'rust';
-- Should return 40
```

## Step 5: Verify Data Integrity

```sql
-- Check that all crates were migrated
SELECT 
    c.name,
    ds.source_name,
    c.total_docs as old_count,
    (SELECT COUNT(*) FROM documents d WHERE d.source_name = c.name) as new_count
FROM crates c
LEFT JOIN document_sources ds ON c.name = ds.source_name
ORDER BY c.name;

-- Test vector search on new schema
SELECT 
    source_name,
    doc_path,
    LEFT(content, 100) as content_preview
FROM documents
WHERE source_name = 'tokio'
ORDER BY embedding <=> (SELECT embedding FROM documents WHERE source_name = 'tokio' LIMIT 1)
LIMIT 5;
```

## Step 6: Update Application Code

Before dropping old tables, update the application to use new schema:

```rust
// Update queries to use 'documents' instead of 'doc_embeddings'
// Update queries to use 'document_sources' instead of 'crates'
```

## Step 7: Test Application Locally

1. Update environment to point to new schema
2. Run test queries
3. Verify all MCP tools work correctly
4. Monitor for any errors

## Step 8: Keep Old Tables (For Now)

```sql
-- Rename old tables instead of dropping (safety measure)
ALTER TABLE crates RENAME TO crates_old;
ALTER TABLE doc_embeddings RENAME TO doc_embeddings_old;

-- Can drop later after confirming everything works
-- DROP TABLE crates_old CASCADE;
-- DROP TABLE doc_embeddings_old CASCADE;
```

## Production Migration (Later)

Once local migration is complete and tested:

```bash
# Simple dump of entire database with new schema
pg_dump -h localhost -U jonathonfritz -d rust_docs_vectors -Fc > doc_server_prod_ready.dump

# On production server
pg_restore -h prod-host -U prod-user -d doc_server_db doc_server_prod_ready.dump
```

## Rollback Plan (If Needed)

If something goes wrong during local migration:

```bash
# Option 1: Restore from backup
dropdb rust_docs_vectors
createdb rust_docs_vectors
pg_restore -h localhost -U jonathonfritz -d rust_docs_vectors ~/backups/rust_docs_[DATE]/rust_docs_vectors_full.dump

# Option 2: If new tables were created but old ones still exist
DROP TABLE documents CASCADE;
DROP TABLE document_sources CASCADE;
-- Continue using old tables
```

## Checklist

### Before Migration
- [ ] Create full backup
- [ ] Verify backup is restorable
- [ ] Review migration SQL
- [ ] Ensure no active connections

### During Migration
- [ ] Create new schema
- [ ] Migrate data
- [ ] Verify counts match
- [ ] Test vector searches
- [ ] Update application code

### After Migration
- [ ] Test all MCP tools
- [ ] Monitor for errors
- [ ] Keep old tables for safety
- [ ] Document any issues

## Quick Commands Reference

```bash
# Backup
pg_dump -h localhost -U jonathonfritz -d rust_docs_vectors -Fc > backup.dump

# Check current state
psql -h localhost -U jonathonfritz -d rust_docs_vectors -c "\dt"

# Run migration
psql -h localhost -U jonathonfritz -d rust_docs_vectors < create_new_schema.sql
psql -h localhost -U jonathonfritz -d rust_docs_vectors < migrate_data.sql

# Verify
psql -h localhost -U jonathonfritz -d rust_docs_vectors -c "
SELECT doc_type, COUNT(*) FROM documents GROUP BY doc_type;"
```