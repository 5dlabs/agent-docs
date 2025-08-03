-- Enable required PostgreSQL extensions for Doc Server
-- This script runs automatically when PostgreSQL starts in Docker

-- Enable pgvector for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Verify extensions are loaded
SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector', 'uuid-ossp');