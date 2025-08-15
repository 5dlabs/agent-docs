# Live Run Checklist (Task 7)

## Pre-flight

- [ ] Backup snapshot taken and restore verified
- [ ] Staging dry-run against fresh prod snapshot completed
- [ ] Zero-downtime plan finalized (additive changes; backfill plan; dual-write if needed)
- [ ] Maintenance window, if required, approved
- [ ] Rollback plan acknowledged (see rollback_plan.md)

## Execute

- [ ] Run K8s migration job (uses `--migrate-only`)
- [ ] Observe logs; ensure validation → apply → record phases succeed

## Post-flight

- [ ] Run `scripts/db_audit.sh` to capture TC-1b snapshot
- [ ] Run performance smoke tests; record p95 latencies
- [ ] Verify pool/config tuning meets objectives
- [ ] Update `docs/perf/summary.md` with measurements

## Sign-off

- [ ] DBA approval
- [ ] App owner approval
