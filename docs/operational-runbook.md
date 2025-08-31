# Doc Server Operational Runbook

## Overview

This runbook provides operational procedures for the Doc Server MCP implementation deployed in production. The server provides semantic search across multiple documentation types including Rust crates, Solana documentation, BirdEye API docs, and other technical resources.

## Service Architecture

- **Service**: doc-server
- **Namespace**: agent-platform
- **Protocol**: MCP over Streamable HTTP (JSON-only)
- **Port**: 3001
- **Database**: PostgreSQL with pgvector extension
- **Deployment**: Kubernetes with auto-scaling (2-10 replicas)

## Quick Reference

### Service URLs
- **Production**: `https://doc-server.agent-platform.svc.cluster.local:3001`
- **Health Check**: `/health`
- **Metrics**: `/metrics`
- **MCP Endpoint**: `/mcp` (POST only)

### Key Commands
```bash
# Check service status
kubectl get pods -n agent-platform -l app=doc-server

# View logs
kubectl logs -n agent-platform -l app=doc-server --tail=100

# Port forward for testing
kubectl port-forward -n agent-platform svc/doc-server-service 3001:3001

# Scale replicas
kubectl scale deployment doc-server -n agent-platform --replicas=5

# Restart deployment
kubectl rollout restart deployment/doc-server -n agent-platform
```

## Alert Response Procedures

### 🔴 CRITICAL: DocServerDown

**Alert**: Service is completely unavailable

**Immediate Actions**:
1. Check pod status:
   ```bash
   kubectl get pods -n agent-platform -l app=doc-server
   ```

2. Check recent events:
   ```bash
   kubectl get events -n agent-platform --sort-by=.metadata.creationTimestamp
   ```

3. View pod logs for errors:
   ```bash
   kubectl logs -n agent-platform -l app=doc-server --tail=50
   ```

4. Check service endpoint:
   ```bash
   kubectl get svc doc-server-service -n agent-platform
   ```

**Common Causes & Solutions**:
- **Pod CrashLoopBackOff**: Check logs for startup errors, verify database connectivity
- **ImagePullBackOff**: Verify image exists in registry, check pull secrets
- **Resource limits**: Check if pods are OOMKilled, increase memory limits if needed
- **Database connectivity**: Verify database is accessible, check connection string

**Escalation**: If service cannot be restored within 10 minutes, escalate to platform team.

### 🟡 WARNING: HighResponseTime

**Alert**: 95th percentile response time > 2 seconds

**Investigation Steps**:
1. Check current response times:
   ```bash
   curl -w "@curl-format.txt" -X POST http://localhost:3001/mcp \
     -H "Content-Type: application/json" \
     -H "MCP-Protocol-Version: 2025-06-18" \
     -d '{"method": "tools/list", "params": {}}'
   ```

2. Monitor database performance:
   ```bash
   # Check database connections
   kubectl exec -it postgres-pod -- psql -U user -d docs -c "SELECT count(*) FROM pg_stat_activity;"
   
   # Check slow queries
   kubectl exec -it postgres-pod -- psql -U user -d docs -c "SELECT query, mean_time FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"
   ```

3. Check resource utilization:
   ```bash
   kubectl top pods -n agent-platform -l app=doc-server
   ```

**Mitigation Actions**:
- Scale up replicas if CPU/memory usage is high
- Check for slow database queries and optimize indexes
- Verify vector search performance
- Monitor embedding service response times

### 🟡 WARNING: HighErrorRate

**Alert**: Error rate > 5% over 3 minutes

**Investigation Steps**:
1. Check error types in logs:
   ```bash
   kubectl logs -n agent-platform -l app=doc-server | grep -E "(ERROR|error|Error)" | tail -20
   ```

2. Test specific tools:
   ```bash
   # Test query tools
   curl -X POST http://localhost:3001/mcp \
     -H "Content-Type: application/json" \
     -H "MCP-Protocol-Version: 2025-06-18" \
     -d '{"method": "tools/call", "params": {"name": "rust_query", "arguments": {"query": "test"}}}'
   ```

3. Check database connectivity:
   ```bash
   curl http://localhost:3001/health
   ```

**Common Error Patterns**:
- **Protocol version errors**: Clients using wrong MCP version
- **Tool execution failures**: Database connectivity or embedding service issues
- **Timeout errors**: Database queries taking too long
- **Memory errors**: Pod running out of memory during large operations

### 🟡 WARNING: DatabaseConnectionFailure

**Alert**: Database connection errors increasing

**Investigation Steps**:
1. Check database pod status:
   ```bash
   kubectl get pods -n databases -l app=vector-postgres
   ```

2. Test database connectivity:
   ```bash
   kubectl exec -it doc-server-pod -- sh -c "pg_isready -h vector-postgres.databases.svc.cluster.local -p 5432"
   ```

3. Check connection pool status:
   ```bash
   # Look for connection pool metrics in logs
   kubectl logs -n agent-platform -l app=doc-server | grep -i "connection\|pool"
   ```

**Resolution Steps**:
- Restart database if unresponsive
- Check network policies between namespaces
- Verify database credentials in secrets
- Scale up database resources if needed

## Maintenance Procedures

### Routine Maintenance

#### Daily Checks
- [ ] Verify service health endpoints respond
- [ ] Check error rates in monitoring dashboard
- [ ] Review overnight logs for any issues
- [ ] Validate all tool functionalities

#### Weekly Maintenance
- [ ] Review performance metrics trends
- [ ] Check disk usage on database
- [ ] Update dependency versions if available
- [ ] Review and archive old logs

#### Monthly Maintenance
- [ ] Performance benchmark review
- [ ] Database maintenance (vacuum, reindex)
- [ ] Security updates
- [ ] Capacity planning review

### Deployment Procedures

#### Standard Deployment
1. **Pre-deployment checks**:
   ```bash
   # Verify tests pass
   ./scripts/acceptance-tests.sh
   
   # Run performance benchmarks
   ./scripts/performance-benchmark.sh
   ```

2. **Deploy via GitHub Actions**:
   - Push to main branch
   - Monitor GitHub Actions workflow
   - Verify deployment succeeds

3. **Post-deployment validation**:
   ```bash
   # Health check
   curl https://doc-server.agent-platform.svc.cluster.local:3001/health
   
   # Tool validation
   ./scripts/acceptance-tests.sh
   ```

#### Emergency Rollback
```bash
# Get previous deployment
kubectl rollout history deployment/doc-server -n agent-platform

# Rollback to previous version
kubectl rollout undo deployment/doc-server -n agent-platform

# Verify rollback
kubectl rollout status deployment/doc-server -n agent-platform
```

### Database Maintenance

#### Database Migrations
```bash
# Run migrations (handled automatically in deployment)
kubectl exec -it doc-server-pod -- ./http_server --migrate-only
```

#### Database Backup
```bash
# Create backup
kubectl exec -it postgres-pod -- pg_dump -U user docs > backup-$(date +%Y%m%d).sql

# Restore backup
kubectl exec -i postgres-pod -- psql -U user docs < backup-20240101.sql
```

#### Performance Optimization
```bash
# Check query performance
kubectl exec -it postgres-pod -- psql -U user -d docs -c "
SELECT query, calls, total_time, mean_time 
FROM pg_stat_statements 
ORDER BY mean_time DESC 
LIMIT 10;"

# Analyze table statistics
kubectl exec -it postgres-pod -- psql -U user -d docs -c "ANALYZE;"
```

## Monitoring and Observability

### Key Metrics
- **Response Time**: 95th percentile < 2 seconds
- **Error Rate**: < 5%
- **Availability**: > 99.9%
- **Concurrent Connections**: Support for 100+
- **Tool Success Rate**: > 95%

### Monitoring Tools
- **Prometheus**: Metrics collection
- **Grafana**: Dashboard visualization
- **Alertmanager**: Alert routing
- **Logs**: Kubernetes logs via kubectl

### Dashboard Access
- **Grafana**: Navigate to "Doc Server Monitoring" dashboard
- **Prometheus**: Query metrics at `/metrics` endpoint
- **Logs**: Use kubectl logs or log aggregation system

## Troubleshooting Guide

### Common Issues

#### "Connection refused" errors
**Symptoms**: Clients cannot connect to service
**Diagnosis**: 
```bash
# Check if pods are running
kubectl get pods -n agent-platform -l app=doc-server

# Check service endpoints
kubectl get endpoints doc-server-service -n agent-platform

# Test connectivity
kubectl exec -it test-pod -- curl http://doc-server-service:3001/health
```

**Resolution**: Restart pods, check network policies, verify service configuration

#### "Protocol version not supported" errors
**Symptoms**: MCP clients receive 400 errors
**Diagnosis**: Check client MCP-Protocol-Version header
**Resolution**: Ensure clients use "2025-06-18" protocol version

#### High memory usage
**Symptoms**: Pods being OOMKilled
**Diagnosis**: 
```bash
kubectl top pods -n agent-platform -l app=doc-server
kubectl describe pod doc-server-pod -n agent-platform
```
**Resolution**: Increase memory limits, optimize queries, check for memory leaks

#### Slow query performance
**Symptoms**: Response times > 2 seconds
**Diagnosis**: Check database query performance, vector search optimization
**Resolution**: Add indexes, optimize vector operations, increase database resources

### Performance Optimization

#### Query Optimization
- Monitor slow queries in PostgreSQL
- Optimize vector similarity search parameters
- Consider query result caching

#### Resource Tuning
- Adjust CPU/memory limits based on usage
- Scale replicas based on load
- Optimize database connection pool size

#### Caching Strategy
- Implement query result caching
- Use connection pooling
- Cache frequently accessed embeddings

## Security Procedures

### Access Control
- Service runs as non-root user
- Network policies restrict traffic
- Secrets managed via Kubernetes secrets

### Security Updates
- Monitor for security vulnerabilities
- Apply patches promptly
- Review access logs regularly

### Incident Response
1. **Detection**: Monitor alerts and logs
2. **Containment**: Isolate affected components
3. **Investigation**: Analyze logs and metrics
4. **Recovery**: Restore service functionality
5. **Post-incident**: Document lessons learned

## Contact Information

### Escalation Contacts
- **Platform Team**: platform-team@company.com
- **Database Team**: database-team@company.com
- **Security Team**: security-team@company.com

### On-call Procedures
- **Primary**: Platform engineer on-call
- **Secondary**: DevOps team lead
- **Emergency**: Infrastructure director

## Documentation Updates

This runbook should be updated when:
- New deployment procedures are implemented
- Alert thresholds are modified
- New monitoring metrics are added
- Service architecture changes

**Last Updated**: [Current Date]
**Next Review**: [Monthly Review Date]