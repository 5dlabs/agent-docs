-- Read-only audit for Task 7 TC-1b
-- Usage: psql "$DATABASE_URL" -f db/sql/audit.sql -v ON_ERROR_STOP=1

\echo '=== Audit: Migration History Table ==='
SELECT to_regclass('public.migration_history') AS migration_history_table;

\echo '=== Audit: Pending/Applied Migrations (if using history table) ==='
SELECT id, version, applied_at, status
FROM migration_history
ORDER BY applied_at DESC NULLS LAST, id
LIMIT 50;

\echo '=== Audit: Foreign Keys (documents -> document_sources) ==='
SELECT
    tc.constraint_name,
    kcu.table_name AS child_table,
    kcu.column_name AS child_column,
    ccu.table_name AS parent_table,
    ccu.column_name AS parent_column
FROM information_schema.table_constraints AS tc
JOIN information_schema.key_column_usage AS kcu
  ON tc.constraint_name = kcu.constraint_name
  AND tc.table_schema = kcu.table_schema
JOIN information_schema.constraint_column_usage AS ccu
  ON ccu.constraint_name = tc.constraint_name
  AND ccu.table_schema = tc.table_schema
WHERE tc.constraint_type = 'FOREIGN KEY'
  AND kcu.table_schema = 'public'
  AND (
    (kcu.table_name = 'documents' AND ccu.table_name = 'document_sources')
    OR (kcu.table_name = 'archived_documents' AND ccu.table_name = 'document_sources')
  )
ORDER BY child_table, constraint_name;

\echo '=== Audit: Indexes (non-vector) on hot paths ==='
SELECT
    i.relname AS index_name,
    t.relname AS table_name,
    pg_get_indexdef(ix.indexrelid) AS index_def
FROM pg_index ix
JOIN pg_class t ON t.oid = ix.indrelid
JOIN pg_class i ON i.oid = ix.indexrelid
JOIN pg_namespace n ON n.oid = t.relnamespace
WHERE n.nspname = 'public'
  AND t.relname IN ('documents','document_sources','archived_documents')
ORDER BY t.relname, i.relname;

\echo '=== Audit: Vector Indexes on embedding (should be none for 3072 dims) ==='
SELECT
    i.relname AS index_name,
    t.relname AS table_name,
    pg_get_indexdef(ix.indexrelid) AS index_def
FROM pg_index ix
JOIN pg_class t ON t.oid = ix.indrelid
JOIN pg_class i ON i.oid = ix.indexrelid
JOIN pg_namespace n ON n.oid = t.relnamespace
WHERE n.nspname = 'public'
  AND t.relname IN ('documents','archived_documents')
  AND pg_get_indexdef(ix.indexrelid) ILIKE '%embedding%';

\echo '=== Audit: Row Counts (quick) ==='
SELECT 'documents' AS table, COUNT(*) AS rows FROM documents
UNION ALL
SELECT 'document_sources' AS table, COUNT(*) AS rows FROM document_sources
UNION ALL
SELECT 'archived_documents' AS table, COUNT(*) AS rows FROM archived_documents;


