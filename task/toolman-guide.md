# Task 1: Database Migration and Schema Harmonization - Tool Usage Guide

## Overview
This guide explains which tools to use and when for completing the database migration from 'rust_docs_vectors' to 'docs' with a harmonized schema supporting multiple documentation types.

## Task Status: COMPLETED ✅

## Tool Usage Strategy

### Phase 1: Environment Analysis and Preparation
**Objective**: Understand current system and prepare for migration

#### Primary Tools
- **`read_file`**: Examine existing database schema, configuration files
- **`search_files`**: Find references to 'rust_docs_vectors' throughout codebase
- **`directory_tree`**: Understand project structure and locate database-related files

#### Specific Actions
```bash
# Use read_file for:
- docker-compose.dev.yml (database configuration)
- .env.example (environment variables)
- crates/database/src/lib.rs (current schema definitions)
- scripts/dev.sh (development setup)

# Use search_files for:
- "rust_docs_vectors" (find all references)
- "DATABASE_URL" (locate connection strings)
- "pgvector" (vector database setup)
- "embedding" (embedding-related code)
```

### Phase 2: Schema Design and Script Creation
**Objective**: Create SQL scripts for the new harmonized schema

#### Primary Tools
- **`write_file`**: Create new SQL migration scripts
- **`edit_file`**: Modify existing schema files and configurations
- **`read_file`**: Reference existing schema for migration mapping

#### Specific Actions
```sql
# Create new schema files using write_file:
- scripts/migration/create_docs_database.sql
- scripts/migration/migrate_data.sql
- scripts/migration/validate_migration.sql

# Key SQL structures to create:
- documents table with 10 doc_type support
- document_sources table for configuration
- Performance indexes
- Data migration procedures
```

### Phase 3: Migration Script Development
**Objective**: Develop robust data migration procedures

#### Primary Tools
- **`write_file`**: Create migration shell scripts and SQL procedures
- **`edit_file`**: Refine migration logic and add error handling
- **`read_multiple_files`**: Compare old and new schema structures

#### Critical Migration Components
```bash
# Create using write_file:
1. Backup procedures (pg_dump scripts)
2. Schema creation scripts
3. Data transformation scripts
4. Validation queries
5. Rollback procedures

# Migration validation queries:
- Row count comparisons
- Embedding dimension verification  
- Metadata transformation validation
- Search functionality testing
```

### Phase 4: Configuration Updates
**Objective**: Update application to use new database

#### Primary Tools
- **`edit_file`**: Update connection strings and configuration
- **`search_files`**: Find all references to update
- **`read_file`**: Verify configuration changes

#### Configuration Files to Update
```bash
# Edit these files:
- docker-compose.dev.yml (database name)
- .env files (DATABASE_URL references)
- Application configuration files
- Development scripts

# Search and replace:
- "rust_docs_vectors" → "docs"
- Update connection strings
- Verify environment variables
```

### Phase 5: Testing and Validation
**Objective**: Ensure migration success and data integrity

#### Primary Tools
- **`read_file`**: Execute validation queries and check results
- **`write_file`**: Create test reports and documentation
- **`search_files`**: Verify no old database references remain

#### Validation Procedures
```sql
# Create validation scripts using write_file:
1. Data integrity checks (row counts, content comparison)
2. Embedding verification (dimension and content)
3. Search functionality tests (query comparisons)
4. Performance benchmarks
5. Schema constraint validation

# Use read_file to verify:
- Migration log files
- Query result comparisons
- Performance test outputs
```

## Tool Selection Rationale

### Local Filesystem Tools Priority
This task primarily requires local file operations since it involves:
- Database schema design (SQL files)
- Configuration updates (YAML, environment files)
- Script development (shell scripts, SQL procedures)
- Documentation creation (markdown files)

### Why Remote Tools Are Not Needed
- **No external research required**: Schema design is based on internal requirements
- **No Kubernetes operations**: This is a local database migration
- **No documentation searches**: Migration procedures are custom-developed
- **No web searches**: All requirements are defined in the task specification

### Tool Efficiency Patterns

#### File Reading Strategy
```bash
# Use read_file for single file examination:
read_file("docker-compose.dev.yml")  # Database configuration
read_file("crates/database/src/lib.rs")  # Current schema

# Use read_multiple_files for comparative analysis:
read_multiple_files([
  "old_schema.sql", 
  "new_schema.sql", 
  "migration_mapping.md"
])
```

#### Search and Update Pattern
```bash
# 1. Search for references
search_files(pattern="rust_docs_vectors", path="/workspace")

# 2. Read each found file
read_file("path/to/file.yml")

# 3. Update with new configuration
edit_file("path/to/file.yml", 
  old_string="rust_docs_vectors", 
  new_string="docs")
```

## Critical Success Factors

### Data Safety Tools Usage
1. **Backup Before Changes**: Use `write_file` to create backup scripts
2. **Incremental Updates**: Use `edit_file` for careful, targeted changes
3. **Validation at Each Step**: Use `read_file` to verify changes

### Performance Monitoring
- Use `write_file` to create performance benchmark scripts
- Use `read_file` to analyze query execution plans
- Track migration progress through log file analysis

### Error Recovery
- Maintain rollback scripts using `write_file`
- Document all changes for potential reversal
- Test rollback procedures before production migration

## Common Pitfalls to Avoid

### File Management Issues
- **Don't**: Create files without understanding current structure
- **Do**: Use `directory_tree` to understand organization first

### Configuration Updates
- **Don't**: Update configurations without backing up originals  
- **Do**: Use `read_file` to capture current state before changes

### Schema Migration
- **Don't**: Migrate data without thorough testing
- **Do**: Create comprehensive validation scripts using `write_file`

## Tool Usage Checklist

### Pre-Migration
- [ ] `directory_tree` - Understand project structure
- [ ] `read_file` - Examine current database configuration
- [ ] `search_files` - Find all database references
- [ ] `read_multiple_files` - Compare schema requirements

### Migration Development  
- [ ] `write_file` - Create SQL schema scripts
- [ ] `write_file` - Develop migration procedures
- [ ] `edit_file` - Refine and optimize scripts
- [ ] `write_file` - Create validation queries

### Configuration Updates
- [ ] `search_files` - Find configuration references
- [ ] `read_file` - Examine current settings
- [ ] `edit_file` - Update database connections
- [ ] `read_file` - Verify configuration changes

### Validation and Testing
- [ ] `write_file` - Create test scripts
- [ ] `read_file` - Execute and analyze test results
- [ ] `write_file` - Document validation outcomes
- [ ] `read_file` - Verify migration completeness

This systematic approach ensures successful database migration while maintaining data integrity and system functionality throughout the process.