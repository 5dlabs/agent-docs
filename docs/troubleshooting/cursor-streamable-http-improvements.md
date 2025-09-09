# MCP Cursor/Streamable HTTP - Troubleshooting Guide Improvements

## Summary of Identified Gaps and Enhancements

Based on my comprehensive analysis of the MCP server implementation, I've identified several areas where the troubleshooting guide can be enhanced to provide more effective diagnostics and solutions.

## 1. Missing Diagnostic Areas

### Session Management Issues
The current guide doesn't address session-related problems:

- **Missing Section**: Session lifecycle troubleshooting
- **Missing Section**: Session timeout configuration issues
- **Missing Section**: Session state validation

**Recommended Addition:**
```markdown
## 10) Session Management Diagnostics

### Check Session Creation
```bash
# Monitor session creation logs
kubectl logs -l app=doc-server -f | grep "session"

# Check session metrics
curl -s http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/metrics | grep session
```

### Validate Session Headers
```bash
# Test with session ID
curl -s -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Mcp-Session-Id: $(uuidgen)" \
  -d '{"method": "tools/list", "params": {}}'
```

### Session Timeout Issues
- Default timeout: 5 minutes
- Check for session expiration in logs
- Verify session cleanup is working
```

### Advanced Logging Techniques
The guide could benefit from more sophisticated logging approaches:

**Recommended Addition:**
```markdown
## 11) Advanced Logging and Tracing

### Enable Comprehensive Request Tracing
```bash
# Enable detailed transport and security logging
RUST_LOG=info,mcp::transport=debug,mcp::security=debug,mcp::headers=debug,mcp::session=debug

# Follow specific request through logs using request_id
kubectl logs -l app=doc-server -f | grep "request_id="
```

### Session Lifecycle Monitoring
```bash
# Monitor session creation and cleanup
kubectl logs -l app=doc-server -f | grep -E "(Created new session|Session expired|Cleaned up.*expired sessions)"
```

### Security Event Tracking
```bash
# Monitor security validation events
kubectl logs -l app=doc-server -f | grep -E "(Security validation|Origin validation|DNS rebinding)"
```
```

## 2. Configuration Troubleshooting Enhancements

### Environment Variable Validation
Add more comprehensive env var checking:

**Recommended Addition:**
```markdown
## 12) Configuration Validation

### Environment Variable Verification
```bash
# Check all required environment variables
kubectl exec -it deployment/doc-server -- env | grep -E "(MCP_|PORT|RUST_LOG)"

# Validate MCP_ENABLE_SSE setting
kubectl exec -it deployment/doc-server -- echo $MCP_ENABLE_SSE

# Check configuration file
kubectl exec -it deployment/doc-server -- cat /app/cto-config.json
```

### Helm Chart Debugging
```bash
# Verify Helm values are applied correctly
helm get values agent-docs -n mcp

# Check deployment environment variables
kubectl describe deployment doc-server | grep -A 20 "Environment:"
```
```

## 3. Protocol-Specific Diagnostics

### Enhanced Protocol Version Testing
Add more comprehensive protocol version validation:

**Recommended Addition:**
```markdown
## 13) Protocol Version Diagnostics

### Test Backwards Compatibility
```bash
# Test with backwards compatible version (should work)
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-03-26" \
  -d '{"method": "tools/list", "params": {}}'

# Test with invalid version (should fail)
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2024-01-01" \
  -d '{"method": "tools/list", "params": {}}'
```

### Header Format Validation
```bash
# Test with malformed headers
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Accept: invalid/format" \
  -d '{"method": "tools/list", "params": {}}'
```
```

## 4. Security-Related Diagnostics

### Enhanced Origin Validation Testing
Add more comprehensive origin testing:

**Recommended Addition:**
```markdown
## 14) Security Validation Diagnostics

### Test Origin Validation
```bash
# Test with allowed localhost origin (should work)
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Origin: http://localhost:3001" \
  -d '{"method": "tools/list", "params": {}}'

# Test with disallowed origin (should fail with 403)
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Origin: https://malicious.com" \
  -d '{"method": "tools/list", "params": {}}'
```

### DNS Rebinding Detection Testing
```bash
# Test DNS rebinding detection (should fail with 403)
curl -s -w "HTTPSTATUS:%{http_code}" \
  -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -H "Host: malicious.com" \
  -H "Origin: http://localhost:3001" \
  -d '{"method": "tools/list", "params": {}}'
```
```

## 5. Performance and Load Testing

### Response Time Analysis
Add performance diagnostics:

**Recommended Addition:**
```markdown
## 15) Performance Diagnostics

### Response Time Testing
```bash
# Test response time distribution
for i in {1..10}; do
  time curl -s -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
    -H "Content-Type: application/json" \
    -H "MCP-Protocol-Version: 2025-06-18" \
    -d '{"method": "tools/list", "params": {}}' > /dev/null
done

# Check server metrics for performance issues
curl -s http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/metrics | grep -E "(response_time|error_rate)"
```

### Concurrent Connection Testing
```bash
# Test concurrent connections (using GNU parallel if available)
seq 1 50 | parallel -j 10 'curl -s -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-06-18" \
  -d '\''{"method": "tools/list", "params": {}}'\'' > /dev/null'
```
```

## 6. Container and Deployment Issues

### Container-Specific Diagnostics
Add container troubleshooting:

**Recommended Addition:**
```markdown
## 16) Container and Deployment Diagnostics

### Check Container Environment
```bash
# Verify container is running
kubectl get pods -l app=doc-server

# Check container logs for startup issues
kubectl logs deployment/doc-server --previous

# Verify environment variables in container
kubectl exec -it deployment/doc-server -- env | grep MCP

# Check resource usage
kubectl top pods -l app=doc-server
```

### Network Policy Verification
```bash
# Check network policies affecting the service
kubectl get networkpolicy -n mcp

# Test service DNS resolution
kubectl exec -it deployment/doc-server -- nslookup doc-server-agent-docs-server.mcp.svc.cluster.local

# Test connectivity to database
kubectl exec -it deployment/doc-server -- nc -zv vector-postgres.databases.svc.cluster.local 5432
```
```

## 7. Client-Side Debugging

### Cursor/Curator Specific Diagnostics
Add IDE client-specific troubleshooting:

**Recommended Addition:**
```markdown
## 17) Cursor/Curator Client Diagnostics

### Client Log Analysis
```bash
# Enable verbose logging in Cursor
# Settings > MCP > Enable Debug Logging

# Check Cursor MCP configuration
# Settings > MCP > Show MCP Server Logs
```

### Network Capture
```bash
# Use browser dev tools to capture network requests
# Look for:
# - Request headers (Origin, Accept, etc.)
# - Response status codes
# - Response headers (Mcp-Session-Id, etc.)
# - Any CORS-related errors
```

### Alternative Client Testing
```bash
# Test with different MCP clients
# - VS Code MCP extension
# - Claude Desktop
# - Custom MCP client implementation
```
```

## 8. Automated Diagnostic Script

### Comprehensive Diagnostic Tool
Create an automated diagnostic script:

**Recommended Addition:**
```markdown
## 18) Automated Diagnostic Script

Use the enhanced acceptance test script for comprehensive diagnostics:

```bash
# Run full diagnostic suite
BASE_URL=http://doc-server-agent-docs-server.mcp.svc.cluster.local:80 ./scripts/diagnostic-suite.sh

# Run specific diagnostic modules
./scripts/diagnostic-suite.sh --module security
./scripts/diagnostic-suite.sh --module performance
./scripts/diagnostic-suite.sh --module session
```
```

## 9. Common Error Patterns

### Error Pattern Recognition
Add common error pattern identification:

**Recommended Addition:**
```markdown
## 19) Common Error Pattern Recognition

### Error Pattern: "Connection Refused"
- **Likely Cause**: Server not running or network connectivity issue
- **Check**: Service health, pod status, network policies

### Error Pattern: "405 Method Not Allowed"
- **Likely Cause**: GET request when SSE disabled, or unsupported method
- **Check**: MCP_ENABLE_SSE setting, HTTP method used

### Error Pattern: "403 Forbidden"
- **Likely Cause**: Origin validation failed or DNS rebinding detected
- **Check**: Origin header, allowed origins configuration, Host/Origin mismatch

### Error Pattern: "406 Not Acceptable"
- **Likely Cause**: Incorrect Accept header for method
- **Check**: Accept header format, method-specific requirements

### Error Pattern: "400 Bad Request"
- **Likely Cause**: Invalid protocol version, malformed JSON, missing headers
- **Check**: MCP-Protocol-Version header, JSON format, required headers
```
```

## 10. Enhanced Data Collection

### Structured Diagnostic Report
Add structured reporting:

**Recommended Addition:**
```markdown
## 20) Structured Diagnostic Reporting

### Generate Diagnostic Report
```bash
# Generate comprehensive diagnostic report
./scripts/generate-diagnostic-report.sh > diagnostic-$(date +%Y%m%d-%H%M%S).txt

# Report includes:
# - Server configuration
# - Network connectivity
# - Security settings
# - Recent error logs
# - Performance metrics
# - Session statistics
```
```

## Implementation Priority

### High Priority (Immediate Value)
1. **Session Management Diagnostics** - Missing critical troubleshooting area
2. **Enhanced Logging Techniques** - Already available, just needs documentation
3. **Configuration Validation** - Essential for deployment issues
4. **Security Validation Testing** - Common source of connection failures

### Medium Priority (Good to Have)
5. **Protocol Version Diagnostics** - Edge case but important for compatibility
6. **Performance Diagnostics** - Useful for production issues
7. **Container Diagnostics** - Kubernetes-specific troubleshooting

### Low Priority (Nice to Have)
8. **Client-Side Debugging** - IDE-specific issues
9. **Automated Diagnostic Script** - Would require development effort
10. **Structured Reporting** - Advanced feature

## Conclusion

The current troubleshooting guide provides a solid foundation but can be significantly enhanced by adding:

1. **Session management diagnostics** - Critical missing area
2. **Advanced logging techniques** - Already implemented, needs documentation
3. **Comprehensive configuration validation** - Essential for deployment issues
4. **Enhanced security diagnostics** - Common failure point
5. **Performance and load testing** - Production readiness

These enhancements would transform the guide from a basic connectivity checker into a comprehensive diagnostic toolkit capable of identifying and resolving the vast majority of MCP connection issues.




