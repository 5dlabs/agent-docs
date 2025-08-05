# Development Setup

This document explains how to quickly start the Doc Server for development.

## Quick Start

### 🚀 Start Development Environment

#### Option 1: Fresh Environment (Empty Database)
```bash
./scripts/dev.sh
```

#### Option 2: With Existing Data (Recommended for Development)
```bash
./scripts/dev.sh --with-data
```

Both scripts will:
1. Start PostgreSQL with pgvector in Docker
2. Wait for the database to be ready
3. Apply database schema and migrations (or load data dump)
4. Start the MCP server on http://localhost:3001

**The `--with-data` option loads a complete database dump containing:**
- 40+ Rust crates with documentation and embeddings
- BirdEye API documentation (600+ endpoints)
- Solana documentation (400+ docs, PDFs, diagrams)
- All vector embeddings ready for semantic search

### 🛑 Stop Development Environment

```bash
./scripts/stop.sh
```

This script will:
1. Stop the PostgreSQL container
2. Kill any running Rust processes
3. Optionally remove the database volume

## Manual Commands

### Database Only

Start just the database (useful if you want to run the server manually):

```bash
docker compose -f docker-compose.dev.yml up -d postgres
```

### Full Production-like Environment

Start everything with Docker Compose (builds the app in Docker too):

```bash
docker compose up -d
```

## Development Workflow

1. **Start the environment**: `./scripts/dev.sh`
2. **Make code changes**: Edit files in `crates/`
3. **Test changes**: The server auto-rebuilds when you restart
4. **Stop when done**: `./scripts/stop.sh`

## Port Configuration

- **MCP Server**: http://localhost:3001
- **Health Check**: http://localhost:3001/health
- **PostgreSQL**: localhost:5433
- **Redis** (optional): localhost:6379

## Database Access

```bash
# Connect to development database
docker compose -f docker-compose.dev.yml exec postgres psql -U docserver -d docs

# Or with psql directly (if installed locally)
psql postgresql://docserver:development_password_change_in_production@localhost:5433/docs
```

## Cursor MCP Configuration

Add to your `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "doc-server": {
      "url": "http://localhost:3001/sse"
    }
  }
}
```

## Troubleshooting

### Database Connection Issues

If you see "role does not exist" errors:
```bash
# Make sure you're using the dev database
unset DATABASE_URL
./scripts/dev.sh
```

### Port Already in Use

If port 3001 is busy:
```bash
# Find what's using the port
lsof -i :3001

# Kill the process
kill -9 <PID>
```

### Docker Issues

```bash
# Reset everything
docker compose -f docker-compose.dev.yml down -v
docker system prune -f
./scripts/dev.sh
```