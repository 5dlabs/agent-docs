# Performance Summary (Task 7)

- Target: p95 < 2s for hot read paths
- Methodology: capture timings before/after migrations; record pool metrics

## Measurements (example placeholders)

- Before: p95 2.8s (N=100)
- After: p95 1.7s (N=100)
- Pool: max=50, idle=10, wait_p95=150ms

## Notes

- Indexes added improved selectivity on `documents(doc_type, source_name)`
- No vector index on 3072-dim `embedding`; rely on metadata filters + brute-force similarity
- Further gains possible with partitioning pruning
