#!/bin/bash

# Backup script for docs database
# Creates timestamped backups of the harmonized multi-type database

set -e

# Configuration - can be overridden via environment variables
DB_NAME="${DB_NAME:-docs}"
DB_USER="${DB_USER:-$(whoami)}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
BACKUP_DIR="${BACKUP_DIR:-$HOME/backups/docs_db}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="$BACKUP_DIR/$TIMESTAMP"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Doc Server Harmonized Database Backup ===${NC}"
echo "Database: $DB_NAME"
echo "Host: $DB_HOST:$DB_PORT"
echo "User: $DB_USER"
echo "Backup location: $BACKUP_PATH"
echo

# Check database connectivity
if ! psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" > /dev/null 2>&1; then
    echo -e "${RED}❌ Cannot connect to database. Please check your configuration.${NC}"
    exit 1
fi

# Create backup directory
mkdir -p "$BACKUP_PATH"

# Create full database backup (binary format)
echo -e "${YELLOW}Creating full database backup (binary format)...${NC}"
pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -Fc > "$BACKUP_PATH/${DB_NAME}_full.dump"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.dump${NC}"

# Create SQL format backup for easy inspection
echo -e "${YELLOW}Creating SQL format backup...${NC}"
pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/${DB_NAME}_full.sql"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.sql${NC}"

# Create compressed SQL backup
echo -e "${YELLOW}Creating compressed SQL backup...${NC}"
pg_dump -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" | gzip > "$BACKUP_PATH/${DB_NAME}_full.sql.gz"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.sql.gz${NC}"

# Export harmonized data as CSV for extra safety
echo -e "${YELLOW}Exporting harmonized data as CSV...${NC}"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" << EOF
\\COPY document_sources TO '$BACKUP_PATH/document_sources_backup.csv' WITH CSV HEADER
\\COPY (SELECT id, doc_type, source_name, doc_path, substring(content, 1, 1000) as content_preview, token_count, created_at, updated_at FROM documents) TO '$BACKUP_PATH/documents_metadata_backup.csv' WITH CSV HEADER
EOF
echo -e "${GREEN}✓ Created: document_sources_backup.csv${NC}"
echo -e "${GREEN}✓ Created: documents_metadata_backup.csv${NC}"

# Get database statistics for the harmonized schema
echo -e "${YELLOW}Capturing database statistics...${NC}"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/db_stats.txt" << 'EOF'
\\echo 'HARMONIZED DATABASE STATISTICS'
\\echo '=============================='
\\echo ''
\\echo 'Table Sizes:'
\\dt+
\\echo ''
\\echo 'Total Row Counts:'
SELECT 'document_sources' as table_name, COUNT(*) as row_count FROM document_sources
UNION ALL
SELECT 'documents', COUNT(*) FROM documents;
\\echo ''
\\echo 'Documents by Type:'
SELECT doc_type, COUNT(*) as doc_count, COUNT(DISTINCT source_name) as sources
FROM documents 
GROUP BY doc_type 
ORDER BY doc_count DESC;
\\echo ''
\\echo 'Sources Summary:'
SELECT doc_type, source_name, total_docs, total_tokens, enabled, last_updated
FROM document_sources 
ORDER BY doc_type, total_docs DESC;
\\echo ''
\\echo 'Embedding Status:'
SELECT 
    doc_type,
    COUNT(*) as total_docs,
    COUNT(embedding) as docs_with_embeddings,
    ROUND(100.0 * COUNT(embedding) / COUNT(*), 2) as embedding_percentage
FROM documents 
GROUP BY doc_type;
\\echo ''
\\echo 'Top 10 Sources by Document Count:'
SELECT source_name, doc_type, COUNT(*) as doc_count 
FROM documents 
GROUP BY source_name, doc_type 
ORDER BY doc_count DESC 
LIMIT 10;
EOF
echo -e "${GREEN}✓ Created: db_stats.txt${NC}"

# Create restore script for the harmonized database
cat > "$BACKUP_PATH/restore.sh" << EOF
#!/bin/bash
# Restore script for harmonized docs database backup

if [ "\$1" = "" ]; then
    echo "Usage: \$0 <target_database_name>"
    echo "Example: \$0 docs_restored"
    echo "Note: This will restore the harmonized multi-type schema"
    exit 1
fi

TARGET_DB=\$1
SCRIPT_DIR="\$( cd "\$( dirname "\${BASH_SOURCE[0]}" )" && pwd )"

echo "This will create a new database: \$TARGET_DB"
echo "This backup contains the harmonized schema supporting multiple doc types."
echo "Press Enter to continue or Ctrl+C to cancel..."
read

# Create database and restore
createdb -h $DB_HOST -p $DB_PORT -U $DB_USER "\$TARGET_DB"
pg_restore -h $DB_HOST -p $DB_PORT -U $DB_USER -d "\$TARGET_DB" "\$SCRIPT_DIR/${DB_NAME}_full.dump"

echo "Restore complete. Verify with:"
echo "psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d \$TARGET_DB -c 'SELECT doc_type, COUNT(*) FROM documents GROUP BY doc_type;'"
EOF
chmod +x "$BACKUP_PATH/restore.sh"
echo -e "${GREEN}✓ Created: restore.sh${NC}"

# Create migration verification script
cat > "$BACKUP_PATH/verify_migration.sh" << 'EOF'
#!/bin/bash
# Verification script to check harmonized database health

DB_NAME="${1:-docs}"

echo "=== Harmonized Database Verification ==="
echo "Database: $DB_NAME"
echo

echo "1. Schema Structure:"
psql -d "$DB_NAME" -c "\\dt"
echo

echo "2. Document Types Supported:"
psql -d "$DB_NAME" -c "SELECT unnest(enum_range(NULL::doc_type)) AS supported_doc_types;"
echo

echo "3. Current Data Distribution:"
psql -d "$DB_NAME" -c "SELECT doc_type, COUNT(*) as documents, COUNT(DISTINCT source_name) as sources FROM documents GROUP BY doc_type ORDER BY documents DESC;"
echo

echo "4. Embedding Coverage:"
psql -d "$DB_NAME" -c "SELECT doc_type, COUNT(*) as total, COUNT(embedding) as with_embeddings, ROUND(100.0 * COUNT(embedding) / COUNT(*), 2) as percentage FROM documents GROUP BY doc_type;"
echo

echo "5. Vector Search Test (if data exists):"
psql -d "$DB_NAME" -c "SELECT source_name, doc_path FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL ORDER BY embedding <=> (SELECT embedding FROM documents WHERE doc_type = 'rust' AND embedding IS NOT NULL LIMIT 1) LIMIT 3;" 2>/dev/null || echo "No data available for vector search test"
EOF
chmod +x "$BACKUP_PATH/verify_migration.sh"
echo -e "${GREEN}✓ Created: verify_migration.sh${NC}"

# Calculate backup size
BACKUP_SIZE=$(du -sh "$BACKUP_PATH" | cut -f1)

echo
echo -e "${GREEN}=== Harmonized Database Backup Complete ===${NC}"
echo "Location: $BACKUP_PATH"
echo "Size: $BACKUP_SIZE"
echo "Files created:"
ls -la "$BACKUP_PATH"
echo
echo -e "${YELLOW}Available restore options:${NC}"
echo "Full restore: cd $BACKUP_PATH && ./restore.sh <new_db_name>"
echo "Manual restore: pg_restore -d <db_name> ${DB_NAME}_full.dump"
echo "SQL restore: psql -d <db_name> -f ${DB_NAME}_full.sql"
echo "Verify database: cd $BACKUP_PATH && ./verify_migration.sh [db_name]"