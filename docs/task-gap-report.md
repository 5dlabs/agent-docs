## Acceptance Criteria Gaps Report (Tasks 5, 7, 13, 14)

Generated to capture remaining gaps versus acceptance criteria for the batched tasks.

### Task 5: Protocol Version Negotiation and Headers

Status: Largely complete for MVP; one notable gap.

- [x] Fixed protocol version constant `2025-06-18` and validation (400 on missing/unsupported)
- [x] Initialize returns `protocolVersion: "2025-06-18"`
- [x] Session state carries fixed protocol version; TTL + cleanup implemented
- [x] Response headers include `MCP-Protocol-Version` (and `Mcp-Session-Id` when a session exists)
- [x] Content-Type validation (accepts `application/json`, `text/event-stream`)
- [x] Transport rejects unsupported methods and wrong/missing headers with proper error
- [x] CORS compatibility via `CorsLayer`
- [ ] Accept header extractor and strict `Accept` validation

Notes:
- Current code validates request `Content-Type` and sets response headers correctly, with solid unit/integration coverage. Strict `Accept` header validation (mentioned in acceptance text) is not implemented; add a small extractor or inline check if required.

### Task 7: Database Migration and Schema Optimization

Status: Partially implemented; several core acceptance items outstanding.

- FR-1 (Migration System)
  - [~] Versioned migration manager exists (`crates/database/src/migration_system.rs`) with history table, transactional apply, status tracking.
  - [ ] Integrated at runtime. Startup uses simple `Migrations::run(...)` rather than the versioned manager.
  - [ ] Rollback plan/documentation for schema migrations (beyond code paths) not present.
  - [x] Migration history tracking implemented by the new manager (not yet used during startup).
  - [x] Atomic operations via transaction during single migration apply.

- FR-2 (Schema Optimization)
  - [x] Non-vector indexes on common columns (e.g., `doc_type`, `source_name`).
  - [ ] Foreign key constraints between `documents` and `document_sources` are not defined.
  - [ ] Partitioning strategy not implemented.
  - [ ] Archive strategy not implemented.
  - [~] Data types are reasonable; no explicit storage optimization review present.

- FR-3 (Performance Optimization)
  - [~] Connection pooling tuning support exists (`pool_config.rs`) with monitoring.
  - [ ] Query performance targets (< 2s) not validated or tested.
  - [ ] DB configuration tuning and index usage validation not demonstrated.
  - [ ] Memory usage targets not validated.

- Live DB Execution Policy
  - [ ] Pre-migration backup + staging dry-run workflows not present.
  - [ ] Zero-downtime migration playbook not documented/implemented.
  - [ ] Post-migration verification checklist not implemented.

- Tooling/Job Integration
  - [ ] `k8s/migration-job.yaml` references `/usr/local/bin/doc-server-migrate`, but our images/binaries currently produce `http_server` (and a CLI under `crates/doc-loader/src/bin/migrate.rs` that would build to a `migrate` binary under that crate). The job is not aligned with the produced image/binary names.

### Task 13: Kubernetes Deployment Configuration (Helm)

Status: Not implemented (beyond chart stubs).

- [ ] Helm chart templates (`templates/`) are missing for Deployment, Service, Ingress, ConfigMap/Secret, PDB, HPA, etc.
- [ ] `values.yaml` is missing (resource requests/limits, envvars, image config, securityContext, autoscaling, etc.).
- [ ] Security policies (PodSecurityContext, NetworkPolicies, etc.) not present.
- [ ] HA and autoscaling config not present.
- [ ] Documentation/deployment guides not present.

Artifacts seen:
- `helm/agent-docs-server/Chart.yaml` and `helm/doc-server/Chart.yaml` exist without accompanying `templates/` or `values.yaml` files.

### Task 14: Container Image Optimization and CI/CD

Status: Optimization largely implemented; CI/CD and security scanning integration missing.

- cargo-chef caching
  - [x] Implemented in `Dockerfile.optimized` (prepare/cook recipe).
  - [ ] Build-time improvement (80%+) not measured/recorded.

- Distroless runtime
  - [x] `gcr.io/distroless/cc-debian12` runtime stage used, non-root user, HEALTHCHECK via app flag, STOPSIGNAL SIGTERM.
  - [~] Shared library requirements appear satisfied given Rust/sqlx usage (no `libpq` linkage), but not explicitly verified.

- Binary optimization/compression
  - [x] Release profile optimized in `Cargo.toml` (size opts, LTO, panic abort, strip).
  - [x] UPX compression applied in builder stage.
  - [ ] Size reduction and performance impact not measured/recorded.

- Graceful shutdown
  - [x] Implemented in `crates/mcp/src/bin/http_server.rs` with SIGINT/SIGTERM handling and 30s note.

- Security scanning pipeline
  - [x] Local script `scripts/scan_image.sh` with Trivy + SARIF + SBOM.
  - [ ] GitHub Actions integration/workflows to run the scan and gate builds are missing.
  - [ ] CI/CD workflow for build/test/push to GHCR and deploy is missing.

### Recommended Next Steps (High-level)

- Task 5: Add optional `Accept` header extractor/validator if strict compliance is desired.
- Task 7: Integrate `DatabaseMigrationManager` into startup, align migration job image/binary, add live migration safeguards/docs, and implement FK/partitioning/archival as per acceptance.
- Task 13: Build out Helm `templates/` and a comprehensive `values.yaml`; wire secrets/env; add HPA/PDB/security contexts; write a deploy README; optionally include chart tests.
- Task 14: Add `.github/workflows` for CI/CD (build, clippy/tests, Docker build with cargo-chef, push to GHCR, Trivy scan gate, Helm deploy); capture image size/benchmark artifacts in CI logs.


