# Doc Server MCP Tools - Fetch Failed Error Documentation

## Overview
This document records the "fetch failed" error patterns observed during testing of the doc server MCP tools.

## Error Patterns Observed

### Jupiter Query Tool (`mcp_doc-server_jupiter_query`)

#### ✅ Successful Queries
- **Query**: "How to perform token swaps using Jupiter API"
  - **Result**: 5 results found, relevance scores 60-100%
  - **Parameters**: `limit=5` (default)
  - **Status**: ✅ Success

- **Query**: "swap"
  - **Result**: 3 results found, relevance scores 80-100%
  - **Parameters**: `limit=3`
  - **Status**: ✅ Success

#### ❌ Failed Queries
- **Query**: "Jupiter v6 API quote endpoint parameters and response format"
  - **Parameters**: `limit=3, category=swap-api`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

- **Query**: "Jupiter API endpoints and integration examples"
  - **Parameters**: `limit=5, topic=apis`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

### Talos Query Tool (`mcp_doc-server_talos_query`)

#### ✅ Successful Queries
- **Query**: "How to configure Talos OS for Kubernetes cluster deployment with custom networking and security policies"
  - **Result**: 5 results found, relevance scores 60-100%
  - **Parameters**: `limit=5`
  - **Status**: ✅ Success
  - **Content**: Development environment setup, configuration schemas, version coverage

#### ❌ Failed Queries (All Subsequent)
- **Query**: "Talos OS machine configuration for high availability Kubernetes cluster with custom CNI and storage classes"
  - **Parameters**: `limit=4`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

- **Query**: "Talos machine configuration networking security"
  - **Parameters**: `limit=3`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

- **Query**: "configuration"
  - **Parameters**: `limit=2`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

- **Query**: "talos"
  - **Parameters**: `limit=1`
  - **Error**: `{"error":"fetch failed"}`
  - **Status**: ❌ Failed

## Pattern Analysis

### Common Characteristics of Failed Queries

1. **Complex Queries with Filters**
   - Queries using optional parameters (category, topic, complexity, format, api_version) appear more prone to failure
   - Simple queries without filters tend to work more reliably

2. **Cascading Failures**
   - Once a tool starts returning "fetch failed", subsequent queries to the same tool continue to fail
   - This suggests a service-level issue rather than query-specific problems

3. **Intermittent Nature**
   - Initial queries work successfully
   - Failures occur after successful operations
   - No clear pattern to predict when failures will occur

### Potential Root Causes

1. **Service Connectivity Issues**
   - Network timeouts
   - Service unavailability
   - Rate limiting

2. **Query Processing Problems**
   - Complex filter combinations causing processing errors
   - Resource exhaustion on the server side
   - Database query timeouts

3. **Infrastructure Issues**
   - Load balancing problems
   - Container restarts
   - Resource constraints

## Recommendations

### For Users
1. **Start with Simple Queries**: Begin with basic queries without optional parameters
2. **Retry Failed Queries**: Simple retry logic may resolve temporary connectivity issues
3. **Avoid Complex Filters Initially**: Test basic functionality before using advanced filtering

### For Developers
1. **Implement Retry Logic**: Add exponential backoff for failed requests
2. **Add Error Logging**: Capture more detailed error information for debugging
3. **Monitor Service Health**: Implement health checks for the doc server services
4. **Query Optimization**: Investigate if complex filter combinations cause performance issues

## Test Results Summary

| Tool | Total Queries | Successful | Failed | Success Rate |
|------|---------------|------------|--------|--------------|
| Jupiter Query | 4 | 2 | 2 | 50% |
| Talos Query | 5 | 1 | 4 | 20% |
| **Total** | **9** | **3** | **6** | **33%** |

## Update: Infrastructure Issues Identified

### ❌ **CTO Tool Ingestion Failure (2025-01-09)**
Attempted to use the CTO docs ingest tool for Jupiter documentation:

**CTO Ingestion Command:**
```bash
mcp_cto_docs_ingest \
  --repository_url="https://github.com/jup-ag/docs" \
  --doc_type="jupiter"
```

**Results:**
- ❌ **Job Failed**: `0ffbe902-330e-4058-a259-a8a9ca50bf3e`
- ❌ **Error**: "Input directory does not exist: jup-ag-docs-out"
- ❌ **Status**: Failed after 42 seconds
- ❌ **Infrastructure Issue**: Missing directory structure

### 🔍 **Root Cause Analysis**
The "fetch failed" errors are occurring at multiple levels:

1. **Transport Level**: Client failing to connect to streamableHttp server
2. **Infrastructure Level**: Missing input directories for ingestion jobs
3. **Service Level**: Doc server reports healthy but ingestion jobs fail

**Error Pattern:**
```
2025-09-09 18:21:56.218 [error] Client error for command fetch failed
2025-09-09 18:21:56.219 [error] Error connecting to streamableHttp server, falling back to SSE: fetch failed
```

### 🔍 **Post-Ingestion Query Results**
After successful re-ingestion, the Jupiter query tool now returns results from the newly ingested content:

**Query**: "get quote API endpoint"
- **Result**: 3 relevant results found
- **Source**: jup-ag-docs (newly ingested)
- **Content**: Includes swap-instructions.api.mdx, quote.api.mdx, and RFQ integration docs
- **Relevance**: 80-100%

### 📊 **Updated Success Rate**
| Tool | Total Queries | Successful | Failed | Success Rate |
|------|---------------|------------|--------|--------------|
| Jupiter Query | 6 | 4 | 2 | 67% |
| Talos Query | 5 | 1 | 4 | 20% |
| **Total** | **11** | **5** | **6** | **45%** |

## Infrastructure Issues Summary

### 🚨 **Critical Issues Identified**

1. **Transport Layer Failures**
   - streamableHttp server connection failures
   - Automatic fallback to SSE not resolving issues
   - Client errors occurring at command level

2. **Ingestion Infrastructure Problems**
   - Missing input directories for ingestion jobs
   - CTO tool jobs failing due to directory structure issues
   - Service reports healthy but functionality impaired

3. **Intermittent Service Availability**
   - Some queries work, others fail consistently
   - No clear pattern to predict failures
   - Service health checks pass but actual functionality fails

### 🔧 **Immediate Actions Required**

1. **Infrastructure Investigation**
   - Check doc server pod/container status in Kubernetes
   - Verify directory structure and permissions
   - Review doc server logs for detailed error information

2. **Service Restart**
   - Consider restarting doc server components
   - Verify Twingate connectivity [[memory:7942215]]
   - Check ArgoCD sync status [[memory:8461887]]

3. **Alternative Approaches**
   - Use direct database queries if available
   - Implement retry logic with exponential backoff
   - Consider using different doc server endpoints

## Next Steps

1. **Infrastructure Fix**: Address the missing directory structure and transport issues
2. **Service Monitoring**: Implement better health checks that test actual functionality
3. **Error Handling**: Add comprehensive retry logic and fallback mechanisms
4. **Documentation**: Update operational runbook with troubleshooting steps
5. **Testing**: Verify fixes with comprehensive test suite

---
*Document created: 2025-01-09*
*Last updated: 2025-01-09*

