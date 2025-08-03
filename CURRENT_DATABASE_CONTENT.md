# Current Database Content

## Overview
The database currently contains **only Rust crate documentation**. There are no other documentation types present yet.

## Statistics
- **Total Crates**: 40
- **Total Embeddings**: 4,133
- **Database Name**: rust_docs_vectors

## Available Rust Crates

The following Rust crates are currently indexed and searchable:

1. **anthropic** - Anthropic API client
2. **anyhow** - Error handling
3. **atomicwrites** - Atomic file writes
4. **axum** - Web framework
5. **chrono** - Date and time
6. **clap** - Command line argument parser
7. **colored** - Terminal colors
8. **config** - Configuration management
9. **dialoguer** - Terminal user interfaces
10. **flamegraph** - Profiling visualization
11. **handlebars** - Template engine
12. **http** - HTTP types
13. **hyper** - HTTP implementation
14. **indicatif** - Progress bars
15. **kube** - Kubernetes client
16. **mdbook** - Book generator
17. **mockall** - Mock object library
18. **opentelemetry** - Observability framework
19. **opentelemetry_sdk** - OpenTelemetry SDK
20. **prometheus** - Metrics
21. **prost** - Protocol Buffers
22. **rand** - Random number generation
23. **redis** - Redis client
24. **regex** - Regular expressions
25. **reqwest** - HTTP client
26. **rmcp** - MCP implementation
27. **rust-mcp-schema** - MCP schema definitions
28. **serde** - Serialization framework
29. **serde_json** - JSON support
30. **serde_yaml** - YAML support
31. **sqlx** - SQL toolkit
32. **tempfile** - Temporary file handling
33. **thiserror** - Error derive macros
34. **tokio** - Async runtime
35. **toml** - TOML support
36. **tonic** - gRPC framework
37. **tower** - Service framework
38. **tracing** - Application tracing
39. **uuid** - UUID generation
40. **wiremock** - HTTP mocking

## Database Schema (Current)

```sql
-- Table: crates
- id, name, version, last_updated, total_docs, total_tokens

-- Table: doc_embeddings  
- id, crate_id, crate_name, doc_path, content, embedding, token_count, created_at
```

## Planned Documentation Types

Based on the architecture discussion, the following documentation types are planned to be added:

1. **BirdEye** - Blockchain API documentation
2. **Jupyter** - Jupyter notebook documentation
3. **GitHub** - Repository documentation (README, docs/)
4. **OpenAPI** - API specifications

## Migration Notes

- All current Rust documentation will be preserved during migration
- The `crate_name` field will become `source_name` in the new schema
- A `doc_type` field will be added with value 'rust' for all existing entries
- Metadata specific to Rust crates (version, features) will move to JSONB field