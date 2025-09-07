# CTO Repo Changes Required - Fix Worker Deployment Redis Connectivity

## Problem Summary
The worker pods in the agent-docs deployment are failing due to DNS resolution issues when trying to connect to a non-existent Redis service.

## Root Cause
The current configuration attempts to connect to:
- `redis://redis-auth-service.databases.svc.cluster.local:6379`

But the actual available Redis service is:
- `redis://rfs-redis.databases.svc.cluster.local:26379`

## Required Changes in CTO Repo

### 1. Update `docs/charts/agent-docs/values.yaml`

**File:** `docs/charts/agent-docs/values.yaml`

**Changes needed:**
```yaml
# Before (lines 21, 35):
DATABASE_URL: ""  # Empty
redisUrl: "redis://redis-auth-service.databases.svc.cluster.local:6379"

# After:
DATABASE_URL: "postgresql://mcp_user:mcp_password@vector-postgres.databases.svc.cluster.local:5432/agent_team"
redisUrl: "redis://rfs-redis.databases.svc.cluster.local:26379"
```

**Specific changes:**
- Line 21: Set DATABASE_URL to PostgreSQL service URL (following production patterns)
- Line 35: Update Redis URL to match actual cluster service and correct port (26379 instead of 6379)

### 2. Verification Steps

After making these changes and letting ArgoCD sync:

1. **Check worker pod status:**
   ```bash
   kubectl -n mcp get pods -l app.kubernetes.io/component=worker
   ```

2. **Verify Redis connectivity:**
   ```bash
   kubectl -n mcp logs <worker-pod-name> --previous
   ```

3. **Test job processing:**
   ```bash
   kubectl -n mcp describe pod <worker-pod-name>
   ```

### 3. Expected Outcome

- Worker pods should transition from `CrashLoopBackOff` to `Running`
- Redis connectivity should be established successfully
- Job processing should work correctly
- Jupiter docs ingestion should proceed without errors

## Additional Context

- **Main server deployment**: ✅ Already working correctly
- **PVC binding**: ✅ Working with `WaitForFirstConsumer`
- **Node affinity**: ✅ Correctly configured for `talos-a43-ee1`
- **Available Redis service**: `rfs-redis.databases.svc.cluster.local:26379` (verified)
- **Available PostgreSQL service**: `vector-postgres.databases.svc.cluster.local:5432` (verified)

## Testing
After deployment, test with Jupiter docs ingestion to verify the queue system works:
```bash
# This should now succeed without DNS resolution errors
curl -X POST http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/ingest \
  -H "Content-Type: application/json" \
  -d '{"doc_type":"jupiter","url":"https://github.com/jup-ag/docs"}'
```

## Impact
These changes will resolve the worker pod failures and restore full functionality to the job queue system, enabling proper document ingestion processing.
