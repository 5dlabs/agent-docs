#!/bin/bash

# Setup script for Doc Server database
# This script creates the new harmonized database and optionally migrates data

set -e  # Exit on any error

# Configuration
DB_NAME="docs"
OLD_DB_NAME="rust_docs_vectors" 
DB_USER="${DB_USER:-$(whoami)}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Setting up Doc Server database...${NC}"

# Function to run SQL and show result
run_sql() {
    local description="$1"
    local sql_file="$2"
    echo -e "${YELLOW}📝 ${description}...${NC}"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$sql_file"
    echo -e "${GREEN}✅ ${description} completed${NC}"
}

# Function to run SQL command directly
run_sql_cmd() {
    local description="$1"
    local sql_cmd="$2"
    echo -e "${YELLOW}📝 ${description}...${NC}"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "$sql_cmd"
    echo -e "${GREEN}✅ ${description} completed${NC}"
}

# Check if PostgreSQL is running
echo -e "${YELLOW}🔍 Checking PostgreSQL connection...${NC}"
if ! psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres -c "SELECT 1;" > /dev/null 2>&1; then
    echo -e "${RED}❌ Cannot connect to PostgreSQL. Please ensure it's running.${NC}"
    echo "Try: docker-compose up postgres -d"
    exit 1
fi

# Check if old database exists for migration
OLD_DB_EXISTS=false
if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -lqt | cut -d \| -f 1 | grep -qw "$OLD_DB_NAME"; then
    OLD_DB_EXISTS=true
    echo -e "${GREEN}✅ Found existing $OLD_DB_NAME database for migration${NC}"
else
    echo -e "${YELLOW}ℹ️  No existing $OLD_DB_NAME database found, skipping migration${NC}"
fi

# Create new database if it doesn't exist
echo -e "${YELLOW}🗄️  Creating database '$DB_NAME'...${NC}"
if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -lqt | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
    echo -e "${YELLOW}⚠️  Database '$DB_NAME' already exists. Continue? (y/N)${NC}"
    read -r response
    if [[ ! "$response" =~ ^[Yy]$ ]]; then
        echo "Exiting..."
        exit 0
    fi
else
    createdb -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$DB_NAME"
    echo -e "${GREEN}✅ Database '$DB_NAME' created${NC}"
fi

# Create schema
run_sql "Creating database schema" "sql/schema.sql"

# Enable dblink for migration (if old database exists)
if [ "$OLD_DB_EXISTS" = true ]; then
    echo -e "${YELLOW}🔗 Enabling dblink for migration...${NC}"
    run_sql_cmd "Enabling dblink extension" "CREATE EXTENSION IF NOT EXISTS dblink;"
    
    echo -e "${YELLOW}📋 Ready to migrate data from $OLD_DB_NAME to $DB_NAME${NC}"
    echo -e "${YELLOW}⚠️  Migration requires manual execution of sql/migrate_from_rust_docs.sql${NC}"
    echo -e "${YELLOW}   Run: psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f sql/migrate_from_rust_docs.sql${NC}"
else
    echo -e "${YELLOW}ℹ️  Skipping migration setup (no source database found)${NC}"
fi

# Show database stats
echo -e "${GREEN}📊 Database setup summary:${NC}"
run_sql_cmd "Checking doc_type enum values" "SELECT unnest(enum_range(NULL::doc_type)) AS doc_types;"
run_sql_cmd "Checking table counts" "
SELECT 
    'documents' as table_name, 
    COUNT(*) as rows 
FROM documents 
UNION ALL 
SELECT 
    'document_sources' as table_name, 
    COUNT(*) as rows 
FROM document_sources;
"

echo -e "${GREEN}🎉 Database setup completed successfully!${NC}"
echo
echo -e "${YELLOW}Next steps:${NC}"
if [ "$OLD_DB_EXISTS" = true ]; then
    echo "1. Run migration: psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f sql/migrate_from_rust_docs.sql"
    echo "2. Verify migration results"
    echo "3. Update your .env file: DATABASE_URL=postgresql://$DB_USER:password@$DB_HOST:$DB_PORT/$DB_NAME"
else
    echo "1. Update your .env file: DATABASE_URL=postgresql://$DB_USER:password@$DB_HOST:$DB_PORT/$DB_NAME"
    echo "2. Start adding documentation sources"
fi