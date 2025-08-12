# Toolman Guide: Database Migration and Schema Optimization

## Tool Selection Rationale
Filesystem tools for creating migration scripts, updating database configuration, and optimizing schema definitions.

## Implementation Approach
1. **read_file**: Examine current schema and database code
2. **write_file**: Create migration framework and scripts
3. **edit_file**: Update database configuration and connections
4. **search_files**: Find all database query locations for optimization

## Key Focus Areas
- **Migration Safety**: Ensure atomic operations and rollback capability
- **Performance**: Optimize indexes and queries for speed
- **Monitoring**: Add comprehensive database health checks
- **Scalability**: Configure for production load requirements