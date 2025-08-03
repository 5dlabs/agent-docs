#!/bin/bash

# Backup script for rust_docs_vectors database
# Creates timestamped backups before migration

set -e

# Configuration
DB_NAME="rust_docs_vectors"
DB_USER="jonathonfritz"
DB_HOST="localhost"
BACKUP_DIR="$HOME/backups/rust_docs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_PATH="$BACKUP_DIR/$TIMESTAMP"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Doc Server Database Backup ===${NC}"
echo "Database: $DB_NAME"
echo "Backup location: $BACKUP_PATH"
echo

# Create backup directory
mkdir -p "$BACKUP_PATH"

# Create full database backup
echo -e "${YELLOW}Creating full database backup...${NC}"
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -Fc > "$BACKUP_PATH/${DB_NAME}_full.dump"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.dump${NC}"

# Also create SQL format for easy inspection
echo -e "${YELLOW}Creating SQL format backup...${NC}"
pg_dump -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/${DB_NAME}_full.sql"
echo -e "${GREEN}✓ Created: ${DB_NAME}_full.sql${NC}"

# Export data as CSV for extra safety
echo -e "${YELLOW}Exporting data as CSV...${NC}"
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" << EOF
\COPY crates TO '$BACKUP_PATH/crates_backup.csv' WITH CSV HEADER
\COPY doc_embeddings TO '$BACKUP_PATH/doc_embeddings_backup.csv' WITH CSV HEADER
EOF
echo -e "${GREEN}✓ Created: crates_backup.csv${NC}"
echo -e "${GREEN}✓ Created: doc_embeddings_backup.csv${NC}"

# Get database statistics
echo -e "${YELLOW}Capturing database statistics...${NC}"
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" > "$BACKUP_PATH/db_stats.txt" << 'EOF'
\echo 'DATABASE STATISTICS'
\echo '=================='
\echo ''
\echo 'Table Sizes:'
\dt+
\echo ''
\echo 'Row Counts:'
SELECT 'crates' as table_name, COUNT(*) as row_count FROM crates
UNION ALL
SELECT 'doc_embeddings', COUNT(*) FROM doc_embeddings;
\echo ''
\echo 'Crate Summary:'
SELECT COUNT(DISTINCT name) as total_crates, SUM(total_docs) as total_docs FROM crates;
\echo ''
\echo 'Top 10 Crates by Document Count:'
SELECT c.name, COUNT(de.id) as doc_count 
FROM crates c 
LEFT JOIN doc_embeddings de ON c.id = de.crate_id 
GROUP BY c.name 
ORDER BY doc_count DESC 
LIMIT 10;
EOF
echo -e "${GREEN}✓ Created: db_stats.txt${NC}"

# Create restore script
cat > "$BACKUP_PATH/restore.sh" << 'EOF'
#!/bin/bash
# Restore script for this backup

if [ "$1" = "" ]; then
    echo "Usage: $0 <target_database_name>"
    echo "Example: $0 rust_docs_vectors_restored"
    exit 1
fi

TARGET_DB=$1
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "This will create a new database: $TARGET_DB"
echo "Press Enter to continue or Ctrl+C to cancel..."
read

createdb "$TARGET_DB"
pg_restore -h localhost -U jonathonfritz -d "$TARGET_DB" "$SCRIPT_DIR/rust_docs_vectors_full.dump"

echo "Restore complete. Verify with:"
echo "psql -d $TARGET_DB -c 'SELECT COUNT(*) FROM doc_embeddings;'"
EOF
chmod +x "$BACKUP_PATH/restore.sh"
echo -e "${GREEN}✓ Created: restore.sh${NC}"

# Calculate backup size
BACKUP_SIZE=$(du -sh "$BACKUP_PATH" | cut -f1)

echo
echo -e "${GREEN}=== Backup Complete ===${NC}"
echo "Location: $BACKUP_PATH"
echo "Size: $BACKUP_SIZE"
echo "Files created:"
ls -la "$BACKUP_PATH"
echo
echo -e "${YELLOW}To restore this backup later:${NC}"
echo "cd $BACKUP_PATH"
echo "./restore.sh <new_database_name>"