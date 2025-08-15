# Database Dump and Restoration

This directory contains a complete dump of the Doc Server database with all ingested documentation.

## Database Contents

The `docs_database_dump.sql.gz` file contains:

- **40+ Rust crates** with full documentation and embeddings
- **BirdEye API documentation** (OpenAPI specs and endpoints)
- **Solana documentation** (markdown, PDFs, architecture diagrams, ZK cryptography specs)
- **Vector embeddings** (3072-dimensional OpenAI text-embedding-3-large)
- **Complete metadata** for all document types

**Total size:** 67MB compressed (184MB uncompressed)
**Total documents:** 4,000+ with embeddings
**Documentation types:** rust, birdeye, solana

## Quick Restoration

### Option 1: Use Development Script (Recommended)

```bash
# This will automatically detect and load the database dump
./scripts/dev.sh --with-data
```

### Option 2: Manual Restoration to Docker Container

```bash
# Start PostgreSQL container
docker compose -f docker-compose.dev.yml up -d postgres

# Wait for it to be ready
sleep 5

# Restore the database
gunzip -c sql/data/docs_database_dump.sql.gz | \
  docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs
```

### Option 3: Manual Restoration to Local PostgreSQL

```bash
# If you have local PostgreSQL and want to restore there
gunzip -c sql/data/docs_database_dump.sql.gz | \
  psql -h localhost -p 5432 -U [your_username] -d docs
```

## Verification

After restoration, verify the data:

```bash
# Check document count
psql -c "SELECT doc_type, COUNT(*) FROM documents GROUP BY doc_type;" [connection_string]

# Test vector search
psql -c "SELECT COUNT(*) FROM documents WHERE embedding IS NOT NULL;" [connection_string]

# Sample query
psql -c "SELECT doc_type, source_name, LEFT(content, 100) FROM documents LIMIT 5;" [connection_string]
```

## Expected Results

You should see approximately:

- **3,000+ Rust documents** from 40+ crates
- **600+ BirdEye API endpoints** with OpenAPI documentation
- **400+ Solana documents** including core docs, architecture diagrams, and ZK specs
- **100% embedding coverage** (all documents have vector embeddings)

## Regenerating the Dump

To create a fresh dump from your local database:

```bash
# Dump from local PostgreSQL
pg_dump -h localhost -p 5432 -U [username] -d docs > sql/data/docs_database_dump.sql

# Compress it
gzip sql/data/docs_database_dump.sql

# Or do both in one command
pg_dump -h localhost -p 5432 -U [username] -d docs | gzip > sql/data/docs_database_dump.sql.gz
```

## Notes

- The dump includes the complete schema (tables, indexes, extensions)
- pgvector extension is automatically included
- No need to run ingestion scripts if you restore this dump
- Embeddings are ready for immediate vector search
- All metadata and relationships are preserved
