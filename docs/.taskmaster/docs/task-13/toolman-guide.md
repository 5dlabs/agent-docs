# Toolman Guide: Kubernetes Deployment Configuration

## Tool Selection Rationale

Kubernetes and filesystem tools for creating Helm charts, configuring deployments, and managing production infrastructure.

## Implementation Approach

1. **read_file**: Examine existing configuration files
2. **write_file**: Create Helm charts and manifests
3. **kubernetes_createResource**: Deploy to cluster
4. **kubernetes_describeResource**: Verify deployment status

## Key Focus Areas

- **Infrastructure as Code**: All configurations versioned and reproducible
- **Security**: Proper RBAC and security policies
- **Scalability**: Auto-scaling and resource optimization
- **Monitoring**: Health checks and observability
