# Doc Server

A high-performance Model Context Protocol (MCP) server that provides AI assistants with semantic search across technical documentation.

## 📚 Documentation

1. **[PROJECT_ARCHITECTURE.md](PROJECT_ARCHITECTURE.md)** - High-level architecture and design decisions
2. **[IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md)** - Detailed implementation with code examples
3. **[CHANGES_FROM_ORIGINAL.md](CHANGES_FROM_ORIGINAL.md)** - Comparison with original implementation
4. **[CURRENT_DATABASE_CONTENT.md](CURRENT_DATABASE_CONTENT.md)** - What's currently in the database

## 🎯 Key Features

- **Multi-Documentation Support**: Rust crates, BirdEye API, Jupyter notebooks, GitHub repos, OpenAPI specs
- **Type-Specific Query Tools**: Dedicated tools for each documentation type
- **Unified Database**: Harmonized schema for all documentation types
- **OpenAI Batching**: 70% cost reduction through batch processing
- **Backward Compatible**: Preserves all existing Rust documentation

## 🚀 Quick Overview

### Documentation Types

#### Currently Available (40 crates in database)
| Type | Query Tool | Source | Status |
|------|------------|---------|---------|
| Rust Crates | `rust_query` | docs.rs | ✅ Active |

#### Planned Extensions
| Type | Query Tool | Source | Ingestion Method |
|------|------------|---------|------------------|
| BirdEye API | `birdeye_query` | API specs | Manual/Agent |
| Jupyter | `jupyter_query` | Notebooks | Manual/Agent |
| GitHub | `github_query` | Repositories | Manual/Agent |
| Others | TBD | Various | Manual/Agent |

### Architecture Highlights

```
┌─────────────┐     ┌─────────────┐
│ Cursor IDE  │     │Claude Code  │
└──────┬──────┘     └──────┬──────┘
       │ MCP Protocol       │
       ▼                    ▼
┌─────────────────────────────────┐
│   Type-Specific Query Tools     │
│  rust_query, birdeye_query...   │
└─────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│    Unified Query Engine         │
│    OpenAI Batch Processor       │
└─────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│  PostgreSQL + pgvector          │
│  Harmonized Document Storage    │
└─────────────────────────────────┘
```

## 🔄 Migration from Original

The new implementation preserves all existing Rust documentation while adding support for multiple documentation types. Key changes:

1. **Tool names**: `query_rust_docs` → `rust_query` (plus new type-specific tools)
2. **Database**: Unified schema replacing separate tables
3. **Embeddings**: OpenAI-only with batching (removed Voyage AI)
4. **Cost**: ~70% reduction through batch processing

## 📋 Implementation Status

- [x] Architecture design
- [x] Database schema design
- [x] Migration strategy
- [ ] Core implementation
- [ ] Testing
- [ ] Deployment

## 🛠️ Next Steps

1. Review the architecture documents
2. Provide feedback on design decisions
3. Begin implementation following the guide

---

*This project extends the original rust-docs-mcp-server to support multiple documentation types while maintaining backward compatibility with existing Rust crate documentation.*