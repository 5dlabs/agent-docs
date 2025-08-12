# Autonomous Agent Prompt: Data Migration and Validation Pipeline

You are tasked with implementing an automated data migration pipeline to populate the database with 184 MB of documentation data, supporting both initial load and incremental migrations with comprehensive validation and rollback capabilities.

## Your Mission

Create a robust data migration system with parallel processing (≥1000 docs/minute), comprehensive validation, checkpointing, rollback procedures, and both CLI and Kubernetes Job interfaces.

## Execution Steps

### Step 1: Create Migration Framework Using doc-loader Crate
- Examine existing `crates/doc-loader` architecture for extension points
- Create migration scripts for each documentation type:
  - Rust crates and standard library documentation
  - Jupyter notebooks and data science content
  - Kubernetes documentation (Cilium, Talos)
  - DeFi protocol documentation (Meteora, Raydium)
  - Systems programming content (eBPF, best practices)
- Design extensible migration framework supporting new document types
- Implement progress tracking with percentage completion and ETA

### Step 2: Implement Blank Database Bootstrap Scenario
- Design initial database setup and schema validation
- Create comprehensive data loading pipeline:
  - Document parsing and metadata extraction
  - Embedding generation with batching optimization
  - Vector storage with pgvector integration
  - Metadata indexing and search optimization
- Implement incremental migration support with checkpointing
- Add migration state persistence and recovery

### Step 3: Add Data Validation and Integrity Checks
- Implement checksum validation for source documents
- Add record count validation and comparison
- Create schema conformance validation:
  - Document structure validation
  - Metadata field validation
  - Vector dimension consistency checks
- Implement duplicate detection and merging strategies
- Add data consistency validation across related records

### Step 4: Implement Parallel Processing and Performance Optimization
- Design parallel processing architecture for ≥1000 docs/minute throughput
- Implement safe concurrency patterns:
  - Document processing parallelization
  - Embedding generation batching
  - Database transaction optimization
- Add backpressure management and resource monitoring
- Implement performance monitoring and bottleneck detection
- Add configurable parallelism based on system resources

### Step 5: Create CLI and Kubernetes Job Interfaces
- Create CLI command for manual migration execution:
  - Progress reporting with detailed status
  - Configuration options for different migration scenarios
  - Validation and dry-run capabilities
- Implement Kubernetes Job template:
  - Resource allocation and limits
  - ConfigMap integration for configuration
  - Status reporting and monitoring integration
- Add migration history persistence in database
- Implement rollback procedures for failed migration batches

## Required Outputs

1. **Migration Framework** with support for all documentation types
2. **Parallel Processing System** achieving ≥1000 docs/minute throughput
3. **Validation Pipeline** with checksums, counts, and schema verification
4. **CLI Interface** with progress reporting and configuration options
5. **Kubernetes Integration** with Job template and monitoring

## Key Technical Requirements

1. **Performance**: ≥1000 documents/minute processing throughput
2. **Reliability**: Rollback capability for any failed migration batch
3. **Validation**: Comprehensive data integrity and consistency checks
4. **Scalability**: Configurable parallelism based on available resources
5. **Monitoring**: Detailed progress reporting with ETA and completion status

## Migration Data Specifications

- **Total Volume**: 184 MB of documentation data
- **Document Types**: Rust, Jupyter, Kubernetes, DeFi, eBPF, Best Practices
- **Processing Target**: Complete migration in < 3 hours
- **Validation Requirements**: 100% data integrity verification
- **Rollback Capability**: Any failed batch must be recoverable

## Tools at Your Disposal

- File system access for migration script creation and data processing
- Database access for schema validation and data operations
- doc-loader crate extension capabilities
- Kubernetes Job creation and monitoring tools

## Success Criteria

Your implementation is complete when:
- Migration pipeline processes 184 MB dataset successfully
- Parallel processing achieves ≥1000 docs/minute throughput
- Validation ensures 100% data integrity with matching checksums
- Rollback procedures work reliably for any failure scenario
- CLI and Kubernetes interfaces provide comprehensive migration control
- Progress reporting enables monitoring and ETA estimation

## Important Implementation Notes

- Implement proper error handling and recovery mechanisms
- Use database transactions for atomic batch operations
- Monitor memory usage during large dataset processing
- Add comprehensive logging for debugging and audit trails
- Test rollback procedures thoroughly before production use

## Performance Targets

- **Throughput**: ≥1000 documents processed per minute
- **Total Time**: 184 MB dataset migration < 3 hours
- **Memory Usage**: Stable memory consumption during processing
- **Error Rate**: < 0.1% processing errors with automatic retry
- **Rollback Time**: Complete batch rollback < 5 minutes

## Validation Commands

```bash
cd /workspace
cargo run --bin migrate -- --validate --dry-run
cargo run --bin migrate -- --full-migration --parallel 8
kubectl apply -f k8s/migration-job.yaml
```

Begin implementation focusing on data integrity, performance, and operational reliability.