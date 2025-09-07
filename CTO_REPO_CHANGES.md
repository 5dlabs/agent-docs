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
- Line 6: Correct image repository from `ghcr.io/5dlabs/agent-docs` to `ghcr.io/5dlabs/agent-docs/server`
- Line 10: Add reference to existing `ghcr-secret` for image pull authentication

### 2. Fix Redis ACL Authentication and Image Repository

**File:** `docs/cto/infra/gitops/databases/redis.yaml`

**Changes needed:**
```yaml
# Add ACL users to Redis configuration:
customConfig:
  - "user worker -@all +@list +@string +@connection +@generic on >workerpass"
  - "user sentinel -@all +ping +sentinel +info +publish +subscribe on >sentinelpass"
```

**File:** `docs/charts/agent-docs/values.yaml`

**Additional changes needed:**
```yaml
# Update image repository (line 6):
image:
  repository: ghcr.io/5dlabs/agent-docs/server  # Add /server suffix

# Add image pull secret (line 10):
imagePullSecrets:
  - name: ghcr-secret  # References existing secret

# Add Redis authentication (new):
env:
  REDIS_USERNAME: worker
  REDIS_PASSWORD: workerpass
```

**Status:** ✅ **Configuration updated!**
- Added ACL users with proper permissions for queue operations
- Worker user has permissions for BRPOP and other queue commands
- Sentinel user has permissions for monitoring operations
- Updated worker configuration to use authentication

**File:** `mcp/src/bin/job_worker.rs`

**Worker code changes:**
```rust
// Added Redis authentication support
let mut con = if let (Ok(username), Ok(password)) = (
    std::env::var("REDIS_USERNAME"),
    std::env::var("REDIS_PASSWORD")
) {
    info!("Connecting to Redis with authentication (user: {})", username);
    let mut con = client.get_multiplexed_async_connection().await?;
    redis::cmd("AUTH")
        .arg(&[&username, &password])
        .query_async::<()>(&mut con)
        .await?;
    con
} else {
    info!("Connecting to Redis without authentication");
    client.get_multiplexed_async_connection().await?
};
```

### 3. Verification Steps

After making these changes and letting ArgoCD sync:

1. **Check migration job status:**
   ```bash
   kubectl -n mcp get jobs
   kubectl -n mcp logs job/agent-docs-server-agent-docs-server-migrations
   ```

2. **Check worker pod status:**
   ```bash
   kubectl -n mcp get pods -l app.kubernetes.io/component=worker
   ```

3. **Verify Redis connectivity:**
   ```bash
   kubectl -n mcp logs <worker-pod-name> --previous
   ```

4. **Test job processing:**
   ```bash
   kubectl -n mcp describe pod <worker-pod-name>
   ```

### 4. Expected Outcome

- Migration job should complete successfully (database schema updates)
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
- **Image registry**: `ghcr.io/5dlabs/agent-docs/server:latest` (corrected path, authentication working)
- **Image pull secret**: `ghcr-secret` (exists, managed by external-secrets)

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
