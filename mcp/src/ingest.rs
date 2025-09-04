use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::server::McpServerState;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct IntelligentIngestRequest {
    pub url: String,
    pub doc_type: String,
    #[serde(default = "default_yes")] // default true
    pub yes: bool,
}

const fn default_yes() -> bool {
    true
}

async fn run_cmd(mut cmd: TokioCommand) -> anyhow::Result<String> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.ok();
        out.push_str(&String::from_utf8_lossy(&buf));
    }
    if let Some(mut stderr) = child.stderr.take() {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.ok();
        out.push_str(&String::from_utf8_lossy(&buf));
    }
    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow::anyhow!(format!(
            "command failed: status={status}, output=\n{}",
            out
        )));
    }
    Ok(out)
}

fn loader_bin() -> std::path::PathBuf {
    std::env::var("LOADER_BIN").map_or_else(
        |_| std::path::PathBuf::from("/app/loader"),
        std::path::PathBuf::from,
    )
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestJobRecord {
    pub id: Uuid,
    pub url: String,
    pub doc_type: String,
    pub status: IngestStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct IngestJobManager {
    inner: Arc<Mutex<HashMap<Uuid, IngestJobRecord>>>,
    ttl: Duration,
    max_entries: usize,
}

impl IngestJobManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            // Default retention: 24h, max 1000 jobs kept in memory
            ttl: Duration::hours(24),
            max_entries: 1000,
        }
    }

    pub async fn get(&self, id: Uuid) -> Option<IngestJobRecord> {
        let map = self.inner.lock().await;
        map.get(&id).cloned()
    }

    pub async fn enqueue(&self, url: String, doc_type: String, yes: bool) -> Uuid {
        // Opportunistic prune before enqueue to enforce limits
        self.prune().await;
        let id = Uuid::new_v4();
        let record = IngestJobRecord {
            id,
            url: url.clone(),
            doc_type: doc_type.clone(),
            status: IngestStatus::Queued,
            started_at: None,
            finished_at: None,
            output: None,
            error: None,
        };
        self.inner.lock().await.insert(id, record);

        let mgr = self.clone();
        tokio::spawn(async move {
            {
                let mut map = mgr.inner.lock().await;
                if let Some(job) = map.get_mut(&id) {
                    job.status = IngestStatus::Running;
                    job.started_at = Some(Utc::now());
                }
            }

            let mut cmd = TokioCommand::new(loader_bin());
            cmd.arg("intelligent").arg(&url);
            if yes {
                cmd.arg("--yes");
            }
            cmd.env("DOC_TYPE_OVERRIDE", &doc_type);

            let result = run_cmd(cmd).await;

            let mut map = mgr.inner.lock().await;
            if let Some(job) = map.get_mut(&id) {
                job.finished_at = Some(Utc::now());
                match result {
                    Ok(output) => {
                        job.status = IngestStatus::Succeeded;
                        job.output = Some(output);
                    }
                    Err(e) => {
                        job.status = IngestStatus::Failed;
                        job.error = Some(e.to_string());
                    }
                }
            }
        });

        id
    }

    /// Start a background cleanup task that prunes old jobs periodically
    pub fn start_cleanup_task(&self) {
        let mgr = self.clone();
        // Run every 5 minutes
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                mgr.prune().await;
            }
        });
    }

    async fn prune(&self) {
        let mut map = self.inner.lock().await;
        if map.is_empty() {
            return;
        }

        let now = Utc::now();
        // 1) Remove finished/failed jobs older than TTL
        let ttl = self.ttl;
        map.retain(|_, job| {
            let finished = job.finished_at.unwrap_or(job.started_at.unwrap_or(now));
            let age = now - finished;
            !(matches!(job.status, IngestStatus::Succeeded | IngestStatus::Failed) && age > ttl)
        });

        // 2) Enforce max_entries by removing oldest finished jobs first
        if map.len() > self.max_entries {
            // Collect ids sorted by (finished_at or started_at)
            let mut items: Vec<(Uuid, DateTime<Utc>, bool)> = map
                .iter()
                .map(|(id, job)| {
                    let ts = job.finished_at.or(job.started_at).unwrap_or_else(Utc::now);
                    // Prefer to drop finished jobs first
                    let is_finished =
                        matches!(job.status, IngestStatus::Succeeded | IngestStatus::Failed);
                    (*id, ts, is_finished)
                })
                .collect();

            // Sort by finished first, then oldest timestamp
            items.sort_by_key(|(_, ts, is_finished)| (u8::from(*is_finished), *ts));

            let to_remove = map.len() - self.max_entries;
            for (id, _, _) in items.into_iter().take(to_remove) {
                map.remove(&id);
            }
        }
    }
}

impl Default for IngestJobManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Enqueue intelligent ingestion and return a job ID immediately.
///
/// # Errors
/// Returns an error response if validation fails.
pub async fn intelligent_ingest_handler(
    State(state): State<McpServerState>,
    Json(body): Json<IntelligentIngestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.url.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "url is required".to_string()));
    }
    if body.doc_type.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "doc_type is required".to_string()));
    }

    let job_id = state
        .ingest_jobs
        .enqueue(body.url, body.doc_type, body.yes)
        .await;

    Ok(Json(json!({ "job_id": job_id })))
}

/// Get the current status of an ingest job.
///
/// # Errors
/// Returns 404 if the job is not found.
pub async fn get_ingest_status_handler(
    State(state): State<McpServerState>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match state.ingest_jobs.get(job_id).await {
        Some(job) => Ok(Json(json!({
            "job_id": job.id,
            "status": job.status,
            "url": job.url,
            "doc_type": job.doc_type,
            "started_at": job.started_at,
            "finished_at": job.finished_at,
            "output": job.output,
            "error": job.error,
        }))),
        None => Err((StatusCode::NOT_FOUND, "job not found".to_string())),
    }
}
