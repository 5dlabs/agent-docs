# Toolman Guide: Task 1 - System Assessment and Migration Planning

## Overview

This task requires comprehensive system analysis and assessment capabilities. The selected tools provide file system access for code review, Kubernetes resource inspection for infrastructure assessment, and documentation query capabilities for understanding existing implementations.

## Core Tools

### Filesystem Server Tools

The filesystem tools are essential for examining the codebase structure and analyzing implementation details.

#### read_file
**Purpose**: Read source code files for detailed analysis
**When to Use**: 
- Examining Rust source files in `crates/` directories
- Reviewing configuration files (Cargo.toml, Dockerfile)
- Analyzing workflow files (.github/workflows/)
**Usage Example**:
```
read_file("/workspace/crates/mcp/src/server.rs")
```

#### list_directory
**Purpose**: Navigate and understand project structure
**When to Use**:
- Discovering crate organization under `crates/`
- Finding configuration and deployment files
- Exploring test directories
**Usage Example**:
```
list_directory("/workspace/crates")
```

#### edit_file
**Purpose**: Document findings and create assessment reports
**When to Use**:
- Creating assessment report files
- Updating documentation with findings
- Adding comments to track issues
**Usage Example**:
```
edit_file("/workspace/assessment_report.md", "# System Assessment\n...")
```

### Kubernetes Tools

These tools help assess the deployment infrastructure and validate Kubernetes configurations.

#### kubernetes_listResources
**Purpose**: List all Kubernetes resources to understand deployment architecture
**When to Use**:
- Discovering deployed services and pods
- Understanding resource organization
- Checking for existing deployments
**Parameters**:
- `type`: Resource type to list (e.g., "pods", "services", "deployments")
- `namespace`: Target namespace (optional)

#### kubernetes_getResource
**Purpose**: Retrieve specific resource configurations
**When to Use**:
- Examining deployment specifications
- Reviewing service configurations
- Checking ConfigMaps and Secrets structure
**Parameters**:
- `type`: Resource type
- `name`: Resource name
- `namespace`: Target namespace

#### kubernetes_describeResource
**Purpose**: Get detailed information about resources including events
**When to Use**:
- Troubleshooting deployment issues
- Understanding resource relationships
- Checking resource health and status
**Parameters**:
- `kind`: Resource kind
- `name`: Resource name
- `namespace`: Target namespace

### Documentation Query Tool

#### rust_query
**Purpose**: Query existing Rust documentation in the system
**When to Use**:
- Understanding documented functionality
- Verifying tool implementation
- Testing query capabilities
**Parameters**:
- `query`: Search query for documentation
- `limit`: Maximum results to return

## Implementation Flow

### Phase 1: Codebase Discovery
1. Use `list_directory` to explore project structure
2. Read `Cargo.toml` files to understand workspace organization
3. Use `read_file` to examine source code in each crate
4. Document findings using `edit_file`

### Phase 2: Infrastructure Assessment
1. Use `kubernetes_listResources` to discover all deployed resources
2. Use `kubernetes_getResource` for specific configurations
3. Use `kubernetes_describeResource` for detailed status
4. Check deployment health and configuration

### Phase 3: Functionality Testing
1. Use `rust_query` to test documentation query capabilities
2. Read test files to understand test coverage
3. Document test results and gaps

### Phase 4: Report Generation
1. Use `edit_file` to create comprehensive assessment report
2. Structure findings by component
3. Include code snippets from `read_file` results
4. Generate migration plan documentation

## Best Practices

### Code Analysis
- Start with high-level structure before diving into details
- Focus on transport-related code for migration assessment
- Check for TODO comments and deprecated warnings
- Review test coverage for critical components

### Infrastructure Review
- Check all namespaces for related resources
- Verify resource specifications match documentation
- Look for missing configurations or resources
- Document any deviations from expected setup

### Documentation
- Create structured reports with clear sections
- Include specific file paths and line numbers
- Provide code snippets for important findings
- Use markdown formatting for readability

## Troubleshooting

### Common Issues

#### File Access Errors
- Ensure correct file paths (absolute paths recommended)
- Check file permissions if access denied
- Verify working directory context

#### Kubernetes Connection Issues
- Confirm cluster connectivity
- Check authentication and permissions
- Verify namespace access rights

#### Large File Handling
- Use `head` and `tail` parameters for large files
- Process files in chunks if needed
- Focus on relevant sections

## Task-Specific Recommendations

### For Transport Assessment
1. Focus on `crates/mcp/src/server.rs` and `transport.rs`
2. Look for HTTP/SSE implementation patterns
3. Compare against MCP 2025-06-18 specification
4. Document all incompatibilities

### For Database Validation
1. Check `crates/database/src/` for schema definitions
2. Review migration files for schema evolution
3. Test vector operations support
4. Verify connection pool configuration

### For Migration Planning
1. Create detailed task breakdown
2. Identify dependencies between components
3. Estimate effort for each migration step
4. Plan rollback strategies

## Output Guidelines

Generate three main documents:

1. **assessment_report.md**: Current state analysis
2. **migration_plan.md**: Step-by-step migration approach
3. **gaps_analysis.md**: Comparison of current vs. required

Each document should include:
- Executive summary
- Detailed findings
- Specific recommendations
- Risk assessment
- Timeline estimates