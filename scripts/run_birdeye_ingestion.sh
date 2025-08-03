#!/bin/bash

# BirdEye Documentation Ingestion Runner
# This script sets up the environment and runs the BirdEye ingestion

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🐦 Starting BirdEye Documentation Ingestion${NC}"

# Check if we're in the right directory
if [ ! -f "scripts/ingestion/ingest_birdeye.py" ]; then
    echo -e "${RED}❌ Please run this script from the project root directory${NC}"
    exit 1
fi

# Load environment variables from .env if it exists
if [ -f ".env" ]; then
    echo -e "${YELLOW}📋 Loading environment variables from .env${NC}"
    export $(cat .env | grep -v '#' | xargs)
else
    echo -e "${YELLOW}⚠️  No .env file found. Ensure DATABASE_URL and OPENAI_API_KEY are set${NC}"
fi

# Check required environment variables
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}❌ DATABASE_URL environment variable is required${NC}"
    echo "Set it in .env or export DATABASE_URL=postgresql://user:pass@localhost:5432/docs"
    exit 1
fi

if [ -z "$OPENAI_API_KEY" ]; then
    echo -e "${RED}❌ OPENAI_API_KEY environment variable is required${NC}"
    echo "Set it in .env or export OPENAI_API_KEY=sk-your-key-here"
    exit 1
fi

# Check if Python dependencies are installed
echo -e "${YELLOW}🔍 Checking Python dependencies...${NC}"
if ! python3 -c "import asyncpg, aiohttp, requests" 2>/dev/null; then
    echo -e "${YELLOW}📦 Installing Python dependencies...${NC}"
    pip3 install -r scripts/ingestion/requirements.txt
    echo -e "${GREEN}✅ Dependencies installed${NC}"
else
    echo -e "${GREEN}✅ Dependencies already installed${NC}"
fi

# Test database connection
echo -e "${YELLOW}🔍 Testing database connection...${NC}"
if ! python3 -c "
import asyncio
import asyncpg
import os
async def test():
    conn = await asyncpg.connect(os.getenv('DATABASE_URL'))
    await conn.execute('SELECT 1')
    await conn.close()
    print('Database connection successful')
asyncio.run(test())
" 2>/dev/null; then
    echo -e "${RED}❌ Cannot connect to database. Please check DATABASE_URL${NC}"
    exit 1
else
    echo -e "${GREEN}✅ Database connection successful${NC}"
fi

# Run the ingestion
echo -e "${GREEN}🚀 Starting BirdEye ingestion...${NC}"
python3 scripts/ingestion/ingest_birdeye.py

echo -e "${GREEN}🎉 BirdEye ingestion completed!${NC}"

# Show results
echo -e "${YELLOW}📊 Checking ingestion results...${NC}"
python3 -c "
import asyncio
import asyncpg
import os
async def check_results():
    conn = await asyncpg.connect(os.getenv('DATABASE_URL'))
    
    # Check document source
    source = await conn.fetchrow('''
        SELECT source_name, total_docs, total_tokens, last_updated 
        FROM document_sources 
        WHERE doc_type = 'birdeye'
    ''')
    
    if source:
        print(f'📋 Document Source: {source[\"source_name\"]}')
        print(f'📄 Total Documents: {source[\"total_docs\"]}')
        print(f'🔤 Total Tokens: {source[\"total_tokens\"]}')
        print(f'🕒 Last Updated: {source[\"last_updated\"]}')
    
    # Check sample documents
    docs = await conn.fetch('''
        SELECT doc_path, token_count 
        FROM documents 
        WHERE doc_type = 'birdeye' 
        ORDER BY created_at 
        LIMIT 5
    ''')
    
    print('\\n📄 Sample Documents:')
    for doc in docs:
        print(f'  - {doc[\"doc_path\"]} ({doc[\"token_count\"]} tokens)')
    
    await conn.close()

asyncio.run(check_results())
"

echo -e "${GREEN}✅ BirdEye documentation ingestion completed successfully!${NC}"