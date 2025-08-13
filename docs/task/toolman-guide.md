# Toolman Guide: Task 18 - Data Migration and Validation Pipeline

## Overview

This task implements a comprehensive data migration pipeline to populate the database with 184 MB of documentation data. The focus is on parallel processing, data validation, rollback capabilities, and both CLI and Kubernetes interfaces.

## Core Tools

### Filesystem Server Tools

#### read_file
**Purpose**: Analyze existing doc-loader architecture and migration patterns
**When to Use**: 
- Examine current doc-loader crate structure for extension points
- Study existing document parsing and processing logic
- Review database schema and migration requirements
- Analyze source documentation structure and formats

#### write_file
**Purpose**: Create migration framework and processing scripts
**When to Use**:
- Implement migration framework extending doc-loader
- Create CLI interface for migration execution
- Write Kubernetes Job templates and configurations
- Add validation and rollback procedure implementations

#### edit_file
**Purpose**: Enhance existing components for migration support
**When to Use**:
- Extend doc-loader crate with migration capabilities
- Modify database connection handling for bulk operations
- Add progress tracking to existing processing pipelines
- Integrate migration functionality with existing codebase

### Database Server Tools

#### query
**Purpose**: Execute database queries for validation and status checking
**When to Use**:
- Validate database schema and table structures
- Check migration progress and completion status
- Verify data integrity with count and checksum queries
- Monitor database performance during migration

#### execute
**Purpose**: Run database operations for migration and rollback
**When to Use**:
- Execute bulk data operations during migration
- Implement rollback procedures for failed batches
- Create indexes and optimize database for migration
- Set up checkpointing and state persistence

#### schema
**Purpose**: Analyze and validate database schema for migrations
**When to Use**:
- Verify pgvector extension and vector dimensions
- Validate table structures and constraints
- Check index configurations for optimal performance
- Ensure schema compatibility with migration data

### Kubernetes Tools

#### kubernetes_getResource
**Purpose**: Examine existing database and infrastructure configuration
**When to Use**:
- Review current PostgreSQL deployment configuration
- Check existing resource limits and scaling policies
- Validate persistent volume and storage configuration
- Examine current monitoring and logging setup

#### kubernetes_listResources
**Purpose**: Discover migration infrastructure and dependencies
**When to Use**:
- Find existing database pods and services
- Locate ConfigMaps and Secrets for database connections
- Identify monitoring and logging infrastructure
- Check for existing migration or batch job resources

## Implementation Flow

### Phase 1: Migration Framework Development
1. Analyze existing doc-loader crate architecture
2. Design extensible migration framework for all document types
3. Implement progress tracking and state persistence
4. Create checkpointing system for resumable migrations

### Phase 2: Parallel Processing Implementation
1. Design worker pool architecture for parallel processing
2. Implement safe concurrency patterns for database operations
3. Add backpressure management and resource monitoring
4. Optimize embedding generation with batching

### Phase 3: Validation and Integrity Systems
1. Implement comprehensive data validation pipeline
2. Add checksum verification and record count validation
3. Create duplicate detection and merging strategies
4. Implement schema conformance validation

### Phase 4: Interface Development
1. Create CLI interface with progress reporting
2. Implement Kubernetes Job template with monitoring
3. Add configuration management and parameter handling
4. Create comprehensive logging and error reporting

### Phase 5: Rollback and Recovery
1. Implement rollback procedures for failed batches
2. Add migration history tracking and state recovery
3. Create disaster recovery and data consistency checks
4. Test failure scenarios and recovery procedures

## Best Practices

### Migration Design
- Process documents in batches to enable rollback
- Use database transactions for atomic batch operations
- Implement idempotent operations for safe retry
- Monitor resource usage and adjust parallelism dynamically

### Data Validation
- Validate data at multiple stages (input, processing, output)
- Use checksums for integrity verification
- Implement schema validation before database insertion
- Check cross-references and relationships between documents

### Performance Optimization
- Use connection pooling optimized for bulk operations
- Batch embedding generation requests efficiently
- Implement parallel processing with appropriate worker counts
- Monitor and adjust based on system resource availability

### Error Handling
- Implement comprehensive error classification and handling
- Provide detailed error messages with remediation guidance
- Log all operations for audit trails and debugging
- Design graceful degradation for partial failures

## Task-Specific Implementation Guidelines

### 1. Migration Framework Architecture
```rust
// Migration framework structure
pub struct MigrationPipeline {
    doc_loader: DocLoader,
    db_pool: Arc<PgPool>,
    embedding_client: Arc<dyn EmbeddingClient>,
    progress_tracker: Arc<ProgressTracker>,
    config: MigrationConfig,
}

impl MigrationPipeline {
    pub async fn execute_migration(
        &self,
        migration_type: MigrationType,
    ) -> Result<MigrationResult> {
        // Implement comprehensive migration logic
    }
    
    pub async fn validate_data(&self) -> Result<ValidationReport> {
        // Implement data validation
    }
    
    pub async fn rollback_batch(&self, batch_id: Uuid) -> Result<()> {
        // Implement rollback procedures
    }
}
```

### 2. Parallel Processing with Worker Pools
```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ParallelProcessor {
    semaphore: Arc<Semaphore>,
    worker_count: usize,
    batch_size: usize,
}

impl ParallelProcessor {
    pub async fn process_documents(
        &self,
        documents: Vec<Document>,
    ) -> Result<Vec<ProcessedDocument>> {
        let semaphore = &self.semaphore;
        let mut handles = Vec::new();
        
        for batch in documents.chunks(self.batch_size) {
            let permit = semaphore.acquire().await?;
            let batch = batch.to_vec();
            
            let handle = tokio::spawn(async move {
                let result = process_batch(batch).await;
                drop(permit); // Release permit
                result
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            results.extend(handle.await??);
        }
        
        Ok(results)
    }
}
```

### 3. Data Validation Pipeline
```rust
#[derive(Debug)]
pub struct ValidationReport {
    pub total_documents: usize,
    pub validated_documents: usize,
    pub failed_validations: Vec<ValidationError>,
    pub checksum_matches: usize,
    pub schema_violations: Vec<SchemaViolation>,
}

pub async fn validate_migration_data(
    db_pool: &PgPool,
    source_path: &Path,
) -> Result<ValidationReport> {
    let mut report = ValidationReport::default();
    
    // Checksum validation
    report.checksum_matches = validate_checksums(db_pool, source_path).await?;
    
    // Record count validation
    let (expected, actual) = validate_record_counts(db_pool, source_path).await?;
    report.total_documents = expected;
    report.validated_documents = actual;
    
    // Schema validation
    report.schema_violations = validate_schema_conformance(db_pool).await?;
    
    Ok(report)
}
```

### 4. CLI Interface Implementation
```rust
// CLI command structure
#[derive(Parser)]
#[command(name = "migrate")]
#[command(about = "Data migration tool for doc-server")]
struct MigrateCli {
    #[command(subcommand)]
    command: MigrateCommand,
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Execute full migration
    Full {
        #[arg(long)]
        parallel: Option<usize>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate existing data
    Validate {
        #[arg(long)]
        repair: bool,
    },
    /// Rollback migration batch
    Rollback {
        batch_id: String,
    },
    /// Show migration status
    Status,
}
```

### 5. Kubernetes Job Template
```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: doc-server-migration
  namespace: default
spec:
  template:
    spec:
      containers:
      - name: migration
        image: doc-server:latest
        command: ["/usr/local/bin/doc-server"]
        args: ["migrate", "full", "--parallel", "8"]
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: postgresql-secret
              key: database-url
        - name: MIGRATION_BATCH_SIZE
          valueFrom:
            configMapKeyRef:
              name: migration-config
              key: batch-size
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
      restartPolicy: Never
  backoffLimit: 3
```

## Troubleshooting

### Migration Performance Issues

#### Slow Processing Speed
- Monitor database connection pool utilization
- Check embedding API rate limits and batching
- Verify parallel worker configuration
- Analyze resource constraints (CPU, memory, I/O)

#### Memory Exhaustion
- Reduce batch sizes for document processing
- Monitor memory usage patterns during migration
- Implement streaming for large document processing
- Check for memory leaks in processing pipelines

#### Database Performance
- Monitor query execution times and optimization
- Check index usage and creation strategies
- Validate connection pool configuration
- Monitor lock contention and transaction times

### Data Validation Failures

#### Checksum Mismatches
- Verify source document integrity
- Check for file corruption during transfer
- Validate encoding and parsing consistency
- Compare original and processed document content

#### Schema Violations
- Check metadata extraction and validation logic
- Verify vector dimension consistency (3072)
- Validate required field presence and types
- Test with sample documents before full migration

### Rollback and Recovery Issues

#### Incomplete Rollback
- Verify transaction boundaries and atomic operations
- Check foreign key constraints and cascade deletions
- Monitor orphaned records and cleanup procedures
- Test rollback procedures with various failure scenarios

#### Checkpoint Recovery
- Validate checkpoint data persistence and consistency
- Check migration state reconstruction logic
- Test resumption from various checkpoint positions
- Monitor progress tracking accuracy during recovery

## Validation Steps

### Development Testing
1. **Unit Tests**: All migration components and validation logic
2. **Integration Tests**: End-to-end migration with sample data
3. **Performance Tests**: Throughput and resource usage validation
4. **Failure Tests**: Rollback and recovery procedure validation

### Production Validation
1. **Full Migration**: Complete 184 MB dataset processing
2. **Performance Validation**: Achieve ≥1000 docs/minute throughput
3. **Data Integrity**: 100% checksum and validation success
4. **Interface Testing**: CLI and Kubernetes interfaces functional

### Quality Assurance
```bash
# Migration testing and validation
cargo test --package doc-loader migration
cargo run --bin migrate -- --validate --dry-run
cargo run --bin migrate -- full --parallel 8

# Kubernetes Job validation
kubectl apply -f k8s/migration-job.yaml --dry-run=client
kubectl logs -f job/doc-server-migration

# Performance benchmarking
cargo test --release migration_performance
```

## Success Indicators

- Migration framework successfully processes all documentation types
- Parallel processing achieves ≥1000 documents/minute throughput
- Data validation ensures 100% integrity with comprehensive checks
- CLI and Kubernetes interfaces provide complete migration control
- Rollback procedures work reliably for any failure scenario
- Migration completes 184 MB dataset within 3 hours
- Progress reporting provides accurate status and ETA
- System handles various failure scenarios gracefully with recovery