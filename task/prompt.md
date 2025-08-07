# Database Migration and Schema Harmonization Task

You are tasked with migrating a PostgreSQL database from 'rust_docs_vectors' to 'docs' with a harmonized schema supporting multiple documentation types. This is a critical foundation task for expanding from a Rust-only documentation server to a multi-type documentation platform.

## Objective
Migrate existing Rust documentation data to a new harmonized schema that supports diverse documentation types including infrastructure tools, blockchain platforms, and programming resources, while preserving all existing data and functionality.

## Current State
- Existing database: `rust_docs_vectors` with Rust crate documentation
- 40+ Rust crates with embeddings and search functionality
- Single-purpose schema designed only for Rust documentation
- Working vector search with semantic queries

## Database Access Information
- **Environment**: Kubernetes cluster (NOT Docker)
- **Pod Name**: `rustdocs-mcp-postgresql-0`
- **Namespace**: `mcp`
- **Connection URL**: `postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql:5432/rust_docs_vectors`
- **Access Method**: Connect via kubectl exec or port-forward to the Kubernetes pod
- **Existing Resources**: Preliminary work and SQL scripts available in the `sql/` folder of the project repository
- **Data Dumps**: Pre-existing documentation dumps available for import

**IMPORTANT**: The database is hosted in a Kubernetes cluster. Do NOT attempt to use Docker commands. Use kubectl to interact with the pod.

## Target State
- New database: `docs` with harmonized schema
- Support for 10 documentation types: rust, jupyter, birdeye, cilium, talos, meteora, solana, ebpf, raydium, rust_best_practices
- All existing Rust data preserved and functional
- Foundation for multi-type documentation ingestion

## Required Schema

### Documents Table
```sql
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    doc_type VARCHAR(50) NOT NULL CHECK (doc_type IN (
        'rust', 'jupyter', 'birdeye', 'cilium', 'talos', 
        'meteora', 'solana', 'ebpf', 'raydium', 'rust_best_practices'
    )),
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072), -- OpenAI text-embedding-3-large
    token_count INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name, doc_path)
);
```

### Document Sources Table
```sql
CREATE TABLE document_sources (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_checked TIMESTAMPTZ,
    last_populated TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);
```

## Implementation Steps

1. **Database Backup**
   - Create full backup of existing 'rust_docs_vectors' database
   - Verify backup integrity and restoration capability

2. **New Database Creation**
   - Create 'docs' database with pgvector extension
   - Implement harmonized schema with proper constraints
   - Add performance indexes for optimal query performance

3. **Data Migration**
   - Map existing Rust documentation to new schema format
   - Preserve all embeddings (3072-dimensional OpenAI vectors)
   - Maintain metadata and relationships
   - Set doc_type='rust' for all migrated documents

4. **Integrity Validation**
   - Verify row counts match between databases
   - Compare sample query results for consistency
   - Test vector search functionality on migrated data
   - Validate all 40 Rust crates remain searchable

5. **Application Update**
   - Update connection strings to use new 'docs' database
   - Verify all existing functionality works with new schema
   - Test search queries and embedding operations

## Critical Requirements

1. **Zero Data Loss**: All existing documents, embeddings, and metadata must be preserved
2. **Functional Preservation**: All search and query functionality must work identically
3. **Performance Maintenance**: Query performance must remain within 10% of original
4. **Schema Validation**: New schema must support all 10 planned documentation types
5. **Rollback Capability**: Maintain ability to rollback to original database if needed

## Validation Criteria

1. **Data Integrity**
   - Document count matches exactly between old and new databases
   - All embeddings transferred with full 3072-dimensional fidelity
   - Metadata preserved in JSONB format
   - No duplicate or missing documents
   
2. **Live Database Verification**
   - **IMPORTANT**: Provide actual query outputs from the live database
   - Execute and show results of verification queries against the actual PostgreSQL instance
   - Include timestamp and row count verification from live database
   - Show sample data retrieved from the migrated database to confirm successful migration
   - Do not just analyze code - interact with the live database and show real outputs

3. **Functional Verification**
   - Vector similarity search produces identical results
   - All 40 Rust crates searchable and accessible
   - Query response times within acceptable range
   - No errors in search operations

4. **Schema Readiness**
   - Constraint validation working for all documentation types
   - JSONB metadata storage functional
   - Indexes properly created for performance
   - Unique constraints preventing duplicates

## Technologies and Tools

- **Database**: PostgreSQL 15+ with pgvector 0.5.0+
- **Migration Tools**: pg_dump, pg_restore, custom SQL scripts
- **Vector Storage**: 3072-dimensional vectors for OpenAI compatibility
- **Metadata Format**: JSONB for flexible type-specific information
- **Backup Strategy**: Full database dumps before any modifications

## Success Metrics

- 100% data preservation (4,133+ documents)
- Identical search result quality and relevance
- Query performance within 10% of baseline
- Zero errors in migrated search functionality
- Schema ready for additional documentation types
- Application successfully connected to new database
- **Live database verification outputs provided** (actual query results, not just code analysis)

Complete this migration with extreme care for data preservation, ensuring that the expanded schema foundation is solid while maintaining all existing functionality without any degradation in performance or capability.

@database-connection for more details