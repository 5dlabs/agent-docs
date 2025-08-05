#!/bin/bash

# Development environment stop script for Doc Server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🛑 Stopping Doc Server Development Environment${NC}"

# Stop development containers
echo -e "${YELLOW}📦 Stopping PostgreSQL database...${NC}"
docker compose -f docker-compose.dev.yml down

# Kill any running cargo processes
echo -e "${YELLOW}🔄 Stopping any running Rust processes...${NC}"
pkill -f "cargo run" 2>/dev/null || echo -e "${YELLOW}⚠️  No running cargo processes found${NC}"

echo -e "${GREEN}✅ Development environment stopped!${NC}"

# Optionally clean up volumes
read -p "Do you want to remove the database volume? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}🗑️  Removing database volume...${NC}"
    docker compose -f docker-compose.dev.yml down -v
    echo -e "${GREEN}✅ Database volume removed!${NC}"
fi