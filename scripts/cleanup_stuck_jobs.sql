-- Cleanup script for stale running jobs after restarts or failures
-- Usage: psql "$DATABASE_URL" -f scripts/cleanup_stuck_jobs.sql

-- Mark stale running crate jobs as failed (no updates in the last 30 minutes)
UPDATE crate_jobs
SET status = 'failed',
    finished_at = CURRENT_TIMESTAMP,
    error = COALESCE(error, '') || CASE WHEN error IS NULL OR error = '' THEN '' ELSE E'\n' END ||
           'Manual cleanup: marked failed due to stale running (updated_at older than 30 minutes)'
WHERE status = 'running'
  AND updated_at < CURRENT_TIMESTAMP - INTERVAL '30 minutes';

-- Mark stale running ingest jobs as failed (no updates in the last 30 minutes)
UPDATE ingest_jobs
SET status = 'failed',
    finished_at = CURRENT_TIMESTAMP,
    error = COALESCE(error, '') || CASE WHEN error IS NULL OR error = '' THEN '' ELSE E'\n' END ||
           'Manual cleanup: marked failed due to stale running (updated_at older than 30 minutes)'
WHERE status = 'running'
  AND updated_at < CURRENT_TIMESTAMP - INTERVAL '30 minutes';

-- Optionally, report how many rows were affected in each table (PostgreSQL psql will show counts)
