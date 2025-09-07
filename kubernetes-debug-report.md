# Kubernetes PVC and Worker Deployment Debug Report

## Executive Summary

The main doc-server deployment is running successfully, but the worker deployment is failing due to DNS resolution issues when trying to connect to a non-existent Redis service.

## Findings

### 1. Deployment Status

**✅ Main Server (Running Successfully):**
- Deployment: `doc-server-agent-docs-server` (namespace: `mcp`)
- Status: 1/1 pods running
- Node: `talos-a43-ee1`
- Node Selector: `kubernetes.io/hostname=talos-a43-ee1`
- Age: 8m7s

**❌ Worker Deployment (Failing):**
- Deployment: `doc-server-agent-docs-server-worker` (namespace: `mcp`)
- Status: 0/1 pods running (2 pods total)
- Node Selector: `kubernetes.io/hostname=talos-a43-ee1`

### 2. Worker Pod Details

**Failing Pod: `doc-server-agent-docs-server-worker-84b57955-jvd58`**
- Status: `CrashLoopBackOff`
- Node: `talos-a43-ee1`
- Age: 8m8s
- Restart Count: 6 times

**Pending Pod: `doc-server-agent-docs-server-worker-57f6f9d487-9fmb9`**
- Status: `Pending`
- Node: `<none>` (not scheduled)
- Age: 14m

### 3. Root Cause Analysis

**Primary Issue: DNS Resolution Failure**

The worker pod is failing with the error:
```
Error: failed to lookup address information: Name or service not known
```

**Expected Redis Service:** `redis-auth-service.databases.svc.cluster.local:6379`

**Actual Redis Services Found:**
- `argocd-redis.argocd.svc.cluster.local:6379` (ArgoCD Redis)
- `rfs-redis.databases.svc.cluster.local:26379` (Redis Sentinel)
- `mailu-redis-master.mailu.svc.cluster.local:6379` (Mailu Redis)

### 4. Configuration Issues

**Code Configuration (mcp/src/queue/mod.rs):**
```rust
pub fn redis_url_from_env() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| {
        "redis://redis-auth-service.databases.svc.cluster.local:6379".to_string()
    })
}
```

**Environment Variable:** `REDIS_URL` is not set, so it defaults to the non-existent service.

### 5. Node Affinity and PVC Status

**✅ Node Scheduling:** Both deployments are correctly configured with:
- Node Selector: `kubernetes.io/hostname=talos-a43-ee1`
- Pods are scheduled to the correct node

**Storage Configuration:** Using `local-path` StorageClass with `volumeBindingMode=WaitForFirstConsumer`

## Recommendations

### Immediate Fix Options

#### Option 1: Update Redis Configuration (Recommended)

Set the correct Redis service URL via environment variable:

```yaml
# In the worker deployment spec
env:
- name: REDIS_URL
  value: "redis://rfs-redis.databases.svc.cluster.local:26379"
```

#### Option 2: Disable Redis Functionality

If Redis is not required for the worker:

```yaml
env:
- name: USE_REDIS_QUEUE
  value: "false"
```

#### Option 3: Create Missing Redis Service

Deploy a Redis service with the expected name:
- Service Name: `redis-auth-service`
- Namespace: `databases`
- Port: `6379`

### Long-term Solutions

1. **Standardize Redis Service Naming:** Use consistent naming conventions across the cluster
2. **Add Health Checks:** Implement Redis connectivity health checks in the deployment
3. **Environment Variable Documentation:** Document all required environment variables
4. **Fallback Configuration:** Implement graceful degradation when Redis is unavailable

## Action Items

### High Priority
- [ ] Fix Redis service URL configuration
- [ ] Verify Redis connectivity
- [ ] Test worker pod startup

### Medium Priority
- [ ] Document environment variable requirements
- [ ] Add Redis health checks to deployment
- [ ] Implement fallback behavior for Redis failures

### Low Priority
- [ ] Standardize Redis service naming across cluster
- [ ] Add monitoring for Redis connectivity issues

## Current Status

- ✅ Main application: Running successfully
- ❌ Worker processes: Failing due to configuration issue
- ✅ Node affinity: Working correctly
- ✅ PVC binding: Working correctly

## Next Steps

1. Choose one of the fix options above
2. Update the deployment configuration
3. Restart the worker deployment
4. Verify both pods start successfully
5. Test the Jupiter docs ingestion job completion

---

*Report generated on: 2025-09-07*
*Debug commands executed against namespace: `mcp`*</content>
</xai:function_call">This file has been created successfully.
