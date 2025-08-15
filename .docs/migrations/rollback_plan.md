# Rollback Plan (Emergency Use Only)

- Preferred strategy is roll-forward fixes. Use rollback only under coordination.
- Preconditions:
  - Verified, recent backup snapshot and tested restore procedure
  - Change window and stakeholder comms ready

## Scope

- Schema-only changes from Task 7 migrations (`001`..`008`).

## Rollback Steps (high level)

1. Quiesce writers or enable dual-write fallback, depending on impact.
2. Apply inverse DDL in a transaction where safe.
   - Drop FKs added in `006_foreign_keys` only if necessary
   - Revert partitioning objects from `007_partitioning`
   - Move data back from `archived_documents` if created in current window
3. Validate schema invariants with `db/sql/audit.sql`.
4. Re-enable traffic; monitor errors and performance.

## Data Safety Notes

- Prefer additive changes; avoid destructive drops.
- For archival, never delete—only move; retain original.

## Verification

- Run `scripts/db_audit.sh` post-rollback
- Confirm app boot and health probes.

## Contacts

- DB admin on-call: <TBD>
- Incident commander: <TBD>
