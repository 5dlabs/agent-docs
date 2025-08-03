# Database Migration Plan

## Overview

This document outlines the migration strategy for moving the Doc Server from the current PostgreSQL instance (`rust_docs_vectors`) to a new production database.

## Current State

- **Database**: `rust_docs_vectors` on localhost
- **Content**: 40 Rust crates with 4,133 embeddings
- **Tables**: `crates`, `doc_embeddings`
- **Vector Extension**: pgvector with 3072-dimensional embeddings

## Target State

- **Database**: New PostgreSQL instance (production)
- **Schema**: Harmonized schema supporting multiple doc types
- **Content**: All existing Rust documentation preserved
- **Additional**: Support for future doc types (BirdEye, Jupyter, etc.)

## Migration Strategy

### Phase 1: Schema Preparation

#### 1.1 Create New Schema on Target Database

```sql
-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create unified schema
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

-- Update trigger function
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

### Phase 2: Data Export

#### 2.1 Export Script

```bash
#!/bin/bash
# export_data.sh

SOURCE_DB="postgresql://jonathonfritz@localhost/rust_docs_vectors"
EXPORT_DIR="./migration_export"

mkdir -p $EXPORT_DIR

echo "Exporting crates table..."
psql $SOURCE_DB -c "\COPY crates TO '$EXPORT_DIR/crates.csv' WITH CSV HEADER"

echo "Exporting doc_embeddings table..."
psql $SOURCE_DB -c "\COPY doc_embeddings TO '$EXPORT_DIR/doc_embeddings.csv' WITH CSV HEADER"

echo "Creating migration SQL..."
cat > $EXPORT_DIR/migration.sql << 'EOF'
-- Temporary tables for import
CREATE TEMP TABLE temp_crates AS SELECT * FROM crates LIMIT 0;
CREATE TEMP TABLE temp_doc_embeddings AS SELECT * FROM doc_embeddings LIMIT 0;

-- Import data
\COPY temp_crates FROM 'crates.csv' WITH CSV HEADER
\COPY temp_doc_embeddings FROM 'doc_embeddings.csv' WITH CSV HEADER

-- Migrate to new schema
INSERT INTO documents (doc_type, source_name, doc_path, content, embedding, token_count, created_at)
SELECT 
    'rust' as doc_type,
    de.crate_name as source_name,
    de.doc_path,
    de.content,
    de.embedding,
    de.token_count,
    de.created_at
FROM temp_doc_embeddings de;

-- Create source configurations
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
FROM temp_crates c;
EOF

echo "Export complete. Files in $EXPORT_DIR/"
```

### Phase 3: Data Import

#### 3.1 Import Process

```bash
#!/bin/bash
# import_data.sh

TARGET_DB="postgresql://user@production-host/doc_server_db"
EXPORT_DIR="./migration_export"

echo "Creating schema on target database..."
psql $TARGET_DB < create_schema.sql

echo "Importing data..."
cd $EXPORT_DIR
psql $TARGET_DB < migration.sql

echo "Verifying migration..."
psql $TARGET_DB << EOF
SELECT 
    doc_type, 
    COUNT(DISTINCT source_name) as sources, 
    COUNT(*) as total_docs 
FROM documents 
GROUP BY doc_type;
EOF
```

### Phase 4: Validation

#### 4.1 Data Integrity Checks

```sql
-- Verify document counts match
SELECT 
    (SELECT COUNT(*) FROM documents WHERE doc_type = 'rust') as new_count,
    (SELECT COUNT(*) FROM rust_docs_vectors.doc_embeddings) as old_count;

-- Verify all crates migrated
SELECT 
    s.source_name,
    COUNT(d.id) as doc_count
FROM document_sources s
LEFT JOIN documents d ON s.source_name = d.source_name AND s.doc_type = d.doc_type
WHERE s.doc_type = 'rust'
GROUP BY s.source_name
ORDER BY s.source_name;

-- Test vector search
SELECT 
    source_name, 
    doc_path, 
    content 
FROM documents 
WHERE doc_type = 'rust' 
    AND source_name = 'tokio'
ORDER BY embedding <=> (SELECT embedding FROM documents LIMIT 1)
LIMIT 5;
```

### Phase 5: Application Updates

#### 5.1 Environment Configuration

```yaml
# Old configuration
MCPDOCS_DATABASE_URL: "postgresql://jonathonfritz@localhost/rust_docs_vectors"

# New configuration
DATABASE_URL: "postgresql://user@production-host/doc_server_db"
```

#### 5.2 Code Updates

```rust
// src/database.rs
pub async fn connect() -> Result<Database, Error> {
    let database_url = env::var("DATABASE_URL")
        .or_else(|_| env::var("MCPDOCS_DATABASE_URL")) // Fallback for compatibility
        .expect("DATABASE_URL must be set");
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    Ok(Database { pool })
}
```

### Phase 6: Cutover Plan

#### 6.1 Zero-Downtime Migration

1. **Preparation** (1 week before)
   - Set up new database with schema
   - Test connectivity from application
   - Run test migrations with sample data

2. **Pre-Migration** (1 day before)
   - Take full backup of source database
   - Notify users of maintenance window
   - Prepare rollback scripts

3. **Migration Day**
   ```bash
   # 1. Put application in read-only mode
   kubectl set env deployment/doc-server READ_ONLY=true
   
   # 2. Final data export
   ./export_data.sh
   
   # 3. Import to new database
   ./import_data.sh
   
   # 4. Run validation checks
   ./validate_migration.sh
   
   # 5. Update application configuration
   kubectl set env deployment/doc-server DATABASE_URL=$NEW_DB_URL
   
   # 6. Remove read-only mode
   kubectl set env deployment/doc-server READ_ONLY-
   
   # 7. Restart pods
   kubectl rollout restart deployment/doc-server
   ```

4. **Post-Migration**
   - Monitor application logs
   - Verify query performance
   - Keep old database for 30 days

### Phase 7: Rollback Plan

If issues arise:

```bash
# 1. Switch back to old database
kubectl set env deployment/doc-server DATABASE_URL=$OLD_DB_URL

# 2. Restart application
kubectl rollout restart deployment/doc-server

# 3. Investigate issues
# 4. Fix and retry migration
```

## Performance Considerations

### Optimization Steps

1. **Before Migration**
   - VACUUM ANALYZE on source database
   - Remove any unused indexes

2. **After Migration**
   - Run ANALYZE on all tables
   - Monitor query performance
   - Adjust work_mem and shared_buffers

3. **pgvector Specific**
   ```sql
   -- Check if we can create indexes (3072 dimensions might be too large)
   -- If not, queries will still work but be slower
   CREATE INDEX CONCURRENTLY idx_documents_embedding 
   ON documents USING ivfflat (embedding vector_cosine_ops)
   WITH (lists = 100);
   ```

## Security Considerations

1. **Connection Security**
   - Use SSL/TLS for database connections
   - Rotate credentials after migration
   - Restrict source IPs

2. **Data Protection**
   - Encrypt exports in transit
   - Delete temporary files after migration
   - Audit access logs

## Timeline

- **Week 1**: Set up target database and test schema
- **Week 2**: Develop and test migration scripts
- **Week 3**: Run test migrations with production data copy
- **Week 4**: Execute production migration
- **Week 5**: Monitor and optimize

## Success Criteria

- [ ] All 40 Rust crates migrated successfully
- [ ] All 4,133 embeddings preserved
- [ ] Vector search functionality working
- [ ] Query performance ≤ 10% degradation
- [ ] Zero data loss
- [ ] Application functioning normally

## Monitoring Post-Migration

Key metrics to track:
- Query response times
- Database connection pool usage
- Vector search performance
- Error rates
- Disk usage growth