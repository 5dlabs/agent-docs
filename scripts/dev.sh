#!/bin/bash

# Development environment startup script for Doc Server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting Doc Server Development Environment${NC}"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for --with-data flag
LOAD_DATA=false
if [[ "$1" == "--with-data" ]]; then
    LOAD_DATA=true
    echo -e "${BLUE}🗂️  Will load database dump with existing documentation${NC}"
fi

# Check for required tools
if ! command_exists docker; then
    echo -e "${RED}❌ Docker is required but not installed. Please install Docker.${NC}"
    exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
    echo -e "${RED}❌ Docker Compose is required but not installed. Please install Docker Compose.${NC}"
    exit 1
fi

# Start the database
echo -e "${YELLOW}📦 Starting PostgreSQL database...${NC}"
docker compose -f docker-compose.dev.yml up -d postgres

# Wait for database to be ready
echo -e "${YELLOW}⏳ Waiting for database to be ready...${NC}"
until docker compose -f docker-compose.dev.yml exec postgres pg_isready -U docserver -d docs; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done

echo -e "${GREEN}✅ Database is ready!${NC}"

# Run migrations
echo -e "${YELLOW}🔄 Running database migrations...${NC}"
if [ -f .env ]; then
    export $(cat .env | xargs)
fi

# Make sure we have the database URL for local development
export DATABASE_URL="postgresql://docserver:development_password_change_in_production@localhost:5433/docs"

# Run Rust migrations
cargo run --bin migrations 2>/dev/null || echo -e "${YELLOW}⚠️  No migrations binary found, skipping Rust migrations${NC}"

# Run SQL schema if it exists (only if not loading data dump)
if [ "$LOAD_DATA" = false ] && [ -f sql/schema.sql ]; then
    echo -e "${YELLOW}📋 Applying SQL schema...${NC}"
    docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs < sql/schema.sql
fi

# Load database dump if requested
if [ "$LOAD_DATA" = true ]; then
    if [ -f sql/data/docs_database_dump.sql.gz ]; then
        echo -e "${YELLOW}📊 Loading database dump with existing documentation...${NC}"
        echo -e "${BLUE}This includes 40+ Rust crates, BirdEye docs, and Solana docs with embeddings${NC}"
        gunzip -c sql/data/docs_database_dump.sql.gz | docker compose -f docker-compose.dev.yml exec -T postgres psql -U docserver -d docs
        echo -e "${GREEN}✅ Database dump loaded successfully!${NC}"
    else
        echo -e "${RED}❌ Database dump not found at sql/data/docs_database_dump.sql.gz${NC}"
        echo -e "${YELLOW}💡 Continuing with empty database...${NC}"
    fi
fi

echo -e "${GREEN}✅ Database setup complete!${NC}"

# Start the MCP server
echo -e "${YELLOW}🖥️  Starting MCP server...${NC}"
echo -e "${BLUE}Server will be available at: http://localhost:3001${NC}"
echo -e "${BLUE}Health check: http://localhost:3001/health${NC}"
echo -e "${BLUE}Press Ctrl+C to stop${NC}"

# Start the server (this will run in foreground)
unset DATABASE_URL
cargo run -p doc-server-mcp --bin http_server