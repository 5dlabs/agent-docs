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


### ⚠️ CRITICAL: DIRECT NETWORK ACCESS TO LIVE DATABASE ⚠️
**NO DOCKER, NO KUBECTL NEEDED**: The database is accessible directly over the network. Connect using the PostgreSQL connection URL directly.

- **Environment**: Live PostgreSQL database in Kubernetes (accessible over network)
- **Host**: `rustdocs-mcp-postgresql.mcp.svc.cluster.local` (full service URL for cross-namespace access)
- **Port**: `5432`
- **Database**: `rust_docs_vectors` (existing), create `docs` (new)
- **Connection URL**: `postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/rust_docs_vectors`
- **Access Method**: **DIRECT NETWORK CONNECTION** - Use psql (installed and available) with the connection URL
- **PostgreSQL Tools**: psql client tools are installed and available for use
- **Data Source**: **IMPORTANT** - The repository contains a dump file with ingested data that MUST be imported into the NEW 'docs' database
- **Existing Resources**: SQL scripts and dump files available in the repository (check `sql/` folder and other locations)

**HOW TO CONNECT**:
```bash
# Direct connection using psql (with full service URL for cross-namespace access)
psql "postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/rust_docs_vectors"

# Or using environment variables
PGPASSWORD=rustdocs123 psql -h rustdocs-mcp-postgresql.mcp.svc.cluster.local -p 5432 -U rustdocs -d rust_docs_vectors

# Python connection example
import psycopg2
conn = psycopg2.connect("postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/rust_docs_vectors")
```

**DO NOT USE**:
- ❌ Docker - Not needed, database is network accessible
- ❌ kubectl - Not needed, database is network accessible
- ❌ Local database files or dumps for verification


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


1. **Database Connection (Direct Network Access)**
   - **DIRECT CONNECTION**: Use psql (installed and available) with the PostgreSQL connection URL
   - `psql "postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/rust_docs_vectors"`
   - This connects to the LIVE database over the network using the full service URL
   - Verify connection to the live PostgreSQL instance

2. **New Database Creation (SEPARATE DATABASE)**
   - Create NEW 'docs' database (separate from rust_docs_vectors)
   - `CREATE DATABASE docs;`
   - Connect to the new database: `psql "postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/docs"`
   - Enable pgvector extension: `CREATE EXTENSION IF NOT EXISTS vector;`

3. **Import Data from Repository Dump**
   - **CRITICAL**: Find and use the dump file included in the repository
   - This dump contains the ingested data that must be imported
   - Import the dump into the NEW 'docs' database (NOT rust_docs_vectors)
   - Example: `psql "postgresql://rustdocs:rustdocs123@rustdocs-mcp-postgresql.mcp.svc.cluster.local:5432/docs" < path/to/dump.sql`
   - Verify all data imported correctly with counts and sample queries

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
   

2. **Live Database Verification - MANDATORY**
   - **CRITICAL REQUIREMENT**: You MUST execute queries against the LIVE Kubernetes database and commit the actual outputs
   - **DO NOT** just read dumps or analyze code - connect to the live PostgreSQL pod in Kubernetes
   - **COMMIT THE FOLLOWING QUERY OUTPUTS** to a file named `live-database-verification.md`:
     
     a) Connection verification:
     ```sql
     -- Show you're connected to the live database
     SELECT current_database(), current_user, inet_server_addr(), version();
     ```
     
     b) Row count from migrated database:
     ```sql
     -- Show actual row count from the new 'docs' database
     SELECT COUNT(*) as total_documents, 
            COUNT(DISTINCT source_name) as unique_sources,
            COUNT(embedding) as documents_with_embeddings
     FROM documents WHERE doc_type = 'rust';
     ```
     
     c) Sample of actual migrated data:
     ```sql
     -- Show 5 actual documents to prove data was migrated
     SELECT id, doc_type, source_name, doc_path, 
            substring(content, 1, 100) as content_preview,
            array_length(embedding, 1) as embedding_dimensions,
            created_at
     FROM documents 
     WHERE doc_type = 'rust' 
     LIMIT 5;
     ```
     
     d) Verification timestamp:
     ```sql
     -- Prove this is live by showing current timestamp
     SELECT NOW() as verification_timestamp, 
            'Live Kubernetes Database' as environment;
     ```
   
   - **PASTE THE ACTUAL OUTPUT** of these queries, not just the queries themselves
   - Include the full psql output showing the connection string and results
   - This proves you're working with the live Kubernetes database, not local files


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


## Required Deliverables

1. **Migration Scripts**: SQL scripts used for the migration
2. **Live Database Verification File** (`live-database-verification.md`):
   - MUST contain actual query outputs from the live Kubernetes database
   - MUST show connection details proving you're on the Kubernetes pod
   - MUST include timestamped verification queries
   - MUST show actual row counts and sample data from the migrated database
   - This file MUST be committed to prove live database interaction


## Success Metrics

- 100% data preservation (4,133+ documents)
- Identical search result quality and relevance
- Query performance within 10% of baseline
- Zero errors in migrated search functionality
- Schema ready for additional documentation types
- Application successfully connected to new database

- **Live database verification file committed with actual query outputs**
- **Proof of connection to Kubernetes pod (not local Docker or dumps)**


Complete this migration with extreme care for data preservation, ensuring that the expanded schema foundation is solid while maintaining all existing functionality without any degradation in performance or capability.

@database-connection for more details