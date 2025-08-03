# Multi-Documentation MCP Server Implementation Guide

## Phase 1: Database Schema Migration

### 1.1 Create New Schema
```sql
-- Create new unified tables
CREATE TABLE documents (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL CHECK (doc_type IN ('rust', 'jupyter', 'birdeye', 'cilium', 'talos', 'meteora', 'solana', 'ebpf', 'rust_best_practices')),
    source_name VARCHAR(255) NOT NULL,
    doc_path TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    embedding vector(3072),
    token_count INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name, doc_path)
);

CREATE TABLE document_sources (
    id SERIAL PRIMARY KEY,
    doc_type VARCHAR(50) NOT NULL,
    source_name VARCHAR(255) NOT NULL,
    config JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_checked TIMESTAMPTZ,
    last_populated TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(doc_type, source_name)
);

-- Create indexes
CREATE INDEX idx_documents_doc_type ON documents(doc_type);
CREATE INDEX idx_documents_source_name ON documents(source_name);
CREATE INDEX idx_documents_type_source ON documents(doc_type, source_name);
CREATE INDEX idx_documents_metadata ON documents USING gin(metadata);

-- Update trigger
CREATE TRIGGER update_documents_updated_at BEFORE UPDATE ON documents 
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_document_sources_updated_at BEFORE UPDATE ON document_sources 
FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

### 1.2 Data Migration
```sql
-- Migrate existing Rust documentation
INSERT INTO documents (doc_type, source_name, doc_path, content, embedding, token_count, created_at)
SELECT 
    'rust' as doc_type,
    crate_name as source_name,
    doc_path,
    content,
    embedding,
    token_count,
    created_at
FROM doc_embeddings;

-- Migrate crate configurations
INSERT INTO document_sources (doc_type, source_name, config, enabled, last_populated)
SELECT 
    'rust' as doc_type,
    c.name as source_name,
    jsonb_build_object(
        'version', c.version,
        'version_spec', COALESCE(cc.version_spec, 'latest'),
        'features', COALESCE(cc.features, ARRAY[]::TEXT[]),
        'expected_docs', c.total_docs
    ) as config,
    COALESCE(cc.enabled, true) as enabled,
    c.last_updated as last_populated
FROM crates c
LEFT JOIN crate_configs cc ON c.name = cc.name;
```

## Phase 2: Core Implementation

### 2.1 OpenAI Batch Processor

```rust
// src/batch_processor.rs
use std::collections::VecDeque;
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct BatchProcessor {
    queue: Arc<Mutex<VecDeque<EmbeddingRequest>>>,
    batch_size: usize,
    rate_limiter: RateLimiter,
}

impl BatchProcessor {
    pub async fn add_to_queue(&self, content: String, metadata: BatchMetadata) {
        let mut queue = self.queue.lock().await;
        queue.push_back(EmbeddingRequest { content, metadata });
        
        if queue.len() >= self.batch_size {
            self.process_batch().await;
        }
    }
    
    async fn process_batch(&self) -> Result<(), Error> {
        let mut queue = self.queue.lock().await;
        let batch: Vec<_> = queue.drain(..self.batch_size.min(queue.len())).collect();
        drop(queue);
        
        // Rate limit check
        self.rate_limiter.wait_if_needed().await?;
        
        // Prepare batch request
        let texts: Vec<String> = batch.iter().map(|r| r.content.clone()).collect();
        
        // Call OpenAI batch API
        let embeddings = self.openai_client
            .embeddings()
            .create(CreateEmbeddingRequestArgs::default()
                .model("text-embedding-3-large")
                .input(texts)
                .build()?)
            .await?;
        
        // Process results
        for (req, embedding) in batch.iter().zip(embeddings.data.iter()) {
            self.store_embedding(req, embedding).await?;
        }
        
        Ok(())
    }
}
```

### 2.2 Query Engine

```rust
// src/query_engine.rs
pub struct QueryEngine {
    db: Arc<Database>,
    llm: Arc<LLMClient>,
}

impl QueryEngine {
    pub async fn query_by_type(
        &self,
        doc_type: &str,
        source_name: Option<&str>,
        question: &str,
        limit: usize,
    ) -> Result<SearchResult, Error> {
        // Generate embedding for the question
        let query_embedding = self.llm.create_embedding(question).await?;
        
        // Vector search with filters
        let sql = r#"
            SELECT id, source_name, doc_path, content, 
                   1 - (embedding <=> $1) as similarity,
                   metadata
            FROM documents
            WHERE doc_type = $2
              AND ($3::text IS NULL OR source_name = $3)
              AND embedding IS NOT NULL
            ORDER BY embedding <=> $1
            LIMIT $4
        "#;
        
        let matches = sqlx::query_as::<_, DocumentMatch>(sql)
            .bind(&query_embedding)
            .bind(doc_type)
            .bind(source_name)
            .bind(limit as i32)
            .fetch_all(&self.db.pool)
            .await?;
        
        // Generate summary
        let summary = self.llm.summarize_results(question, &matches).await?;
        
        Ok(SearchResult {
            matches,
            summary,
            tokens_used: summary.tokens_used,
        })
    }
}
```

### 2.3 MCP Tool Handlers

```rust
// src/tools/rust_query.rs
pub async fn rust_query_handler(params: Value) -> Result<Value, Error> {
    let args: RustQueryArgs = serde_json::from_value(params)?;
    
    let result = query_engine
        .query_by_type("rust", Some(&args.crate_name), &args.question, 5)
        .await?;
    
    Ok(json!({
        "matches": result.matches,
        "summary": result.summary,
        "tokens_used": result.tokens_used
    }))
}

// src/tools/birdeye_query.rs
pub async fn birdeye_query_handler(params: Value) -> Result<Value, Error> {
    let args: BirdEyeQueryArgs = serde_json::from_value(params)?;
    
    // BirdEye queries might filter by API endpoint in metadata
    let result = query_engine
        .query_by_type("birdeye", None, &args.question, 5)
        .await?;
    
    // Filter results by API name if provided
    let filtered_matches = if let Some(api_name) = args.api_name {
        result.matches.into_iter()
            .filter(|m| {
                m.metadata.get("endpoint")
                    .and_then(|e| e.as_str())
                    .map(|e| e.contains(&api_name))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        result.matches
    };
    
    Ok(json!({
        "matches": filtered_matches,
        "summary": result.summary,
        "tokens_used": result.tokens_used
    }))
}
```

## Phase 3: Document Loaders

### 3.1 GitHub Loader
```rust
// src/loaders/github_loader.rs
pub struct GitHubLoader {
    client: octocrab::Octocrab,
    batch_processor: Arc<BatchProcessor>,
}

impl GitHubLoader {
    pub async fn load_repository(&self, owner: &str, repo: &str) -> Result<(), Error> {
        // Fetch README
        let readme = self.client
            .repos(owner, repo)
            .get_readme()
            .send()
            .await?;
        
        // Fetch docs directory
        let docs_contents = self.client
            .repos(owner, repo)
            .get_content()
            .path("docs")
            .send()
            .await?;
        
        // Process each document
        for item in docs_contents.items {
            if item.name.ends_with(".md") {
                let content = self.fetch_file_content(&item.download_url).await?;
                let chunks = chunk_markdown(&content, 1000);
                
                for (idx, chunk) in chunks.iter().enumerate() {
                    let metadata = json!({
                        "repo": format!("{}/{}", owner, repo),
                        "file": item.path,
                        "chunk_index": idx
                    });
                    
                    self.batch_processor.add_to_queue(
                        chunk.clone(),
                        BatchMetadata {
                            doc_type: "github".to_string(),
                            source_name: format!("{}/{}", owner, repo),
                            doc_path: format!("{}#chunk{}", item.path, idx),
                            metadata,
                        }
                    ).await;
                }
            }
        }
        
        Ok(())
    }
}
```

### 3.2 API-Specific Loaders
Each API will have its own custom loader based on its format and structure. For example:

```rust
// src/loaders/birdeye_loader.rs
pub struct BirdEyeLoader {
    batch_processor: Arc<BatchProcessor>,
}

// Implementation will be specific to BirdEye's API documentation format
```

## Phase 4: Rate Limiting

```rust
// src/rate_limiter.rs
use governor::{Quota, RateLimiter as Governor};

pub struct RateLimiter {
    rpm_limiter: Governor<NotKeyed, InMemoryState, MonotonicClock>,
    tpm_limiter: Governor<NotKeyed, InMemoryState, MonotonicClock>,
}

impl RateLimiter {
    pub fn new() -> Self {
        // 3000 requests per minute
        let rpm_quota = Quota::per_minute(nonzero!(3000u32));
        let rpm_limiter = Governor::new(rpm_quota);
        
        // 1M tokens per minute (estimate ~250 tokens per embedding request)
        let tpm_quota = Quota::per_minute(nonzero!(4000u32)); // ~1M tokens
        let tpm_limiter = Governor::new(tpm_quota);
        
        Self { rpm_limiter, tpm_limiter }
    }
    
    pub async fn wait_if_needed(&self) -> Result<(), Error> {
        self.rpm_limiter.until_ready().await;
        self.tpm_limiter.until_ready().await;
        Ok(())
    }
}
```

## Phase 5: Testing Strategy

### 5.1 Integration Tests
```rust
#[tokio::test]
async fn test_multi_doc_query() {
    let server = setup_test_server().await;
    
    // Add test data for different doc types
    add_test_rust_docs(&server).await;
    add_test_github_docs(&server).await;
    
    // Test type-specific queries
    let rust_result = server.handle_tool_call("rust_query", json!({
        "crate_name": "tokio",
        "question": "How to use select! macro?"
    })).await.unwrap();
    
    assert!(rust_result["matches"].as_array().unwrap().len() > 0);
    
    // Test cross-doc search
    let all_result = server.handle_tool_call("search_all", json!({
        "question": "async programming"
    })).await.unwrap();
    
    assert!(all_result["results_by_type"]["rust"].as_array().unwrap().len() > 0);
}
```

## Phase 6: Deployment Updates

### 6.1 Helm Values Update
```yaml
app:
  env:
    - name: OPENAI_API_KEY
      valueFrom:
        secretKeyRef:
          name: rustdocs-secrets
          key: openai-api-key
    - name: BATCH_SIZE
      value: "100"
    - name: RATE_LIMIT_RPM
      value: "3000"
    - name: SUPPORTED_DOC_TYPES
      value: "rust,jupyter,birdeye,cilium,talos,meteora,solana,ebpf,rust_best_practices"
```

### 6.2 Migration Job
```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: schema-migration
spec:
  template:
    spec:
      containers:
      - name: migrate
        image: rust-docs-mcp:latest
        command: ["cargo", "run", "--bin", "migrate_schema"]
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: postgres-credentials
              key: url
```

## Implementation Timeline

1. **Week 1**: Database schema migration and data preservation
2. **Week 2**: Core query engine and batch processor
3. **Week 3**: MCP tool handlers and integration
4. **Week 4**: Document loaders for non-Rust types
5. **Week 5**: Testing and deployment
6. **Week 6**: Performance optimization and monitoring

## Key Considerations

1. **Backward Compatibility**: Existing Rust documentation queries must continue working
2. **Cost Management**: Batch processing reduces OpenAI API costs by ~70%
3. **Performance**: Vector indexes may need tuning for 3072-dimension embeddings
4. **Extensibility**: Adding new doc types should require minimal code changes
5. **Monitoring**: Track embedding costs, query latency, and success rates per doc type