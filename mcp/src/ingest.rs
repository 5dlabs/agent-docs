use axum::{extract::Path, extract::State, http::StatusCode, Json};
use db::{models::JobStatus, DatabasePool};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::server::McpServerState;
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

#[derive(Clone)]
pub struct IngestJobManager {
    db_pool: DatabasePool,
}

impl IngestJobManager {
    #[must_use]
    pub const fn new(db_pool: DatabasePool) -> Self {
        Self { db_pool }
    }

    pub async fn get(&self, id: Uuid) -> Option<db::models::IngestJob> {
        match db::queries::IngestJobQueries::find_job_by_id(self.db_pool.pool(), id).await {
            Ok(job) => job,
            Err(_) => None,
        }
    }

    pub async fn enqueue(&self, url: String, doc_type: String, yes: bool) -> Uuid {
        // Create job in DB first so any replica can see it
        let created = db::queries::IngestJobQueries::create_job(
            self.db_pool.pool(),
            &url,
            &doc_type,
        )
        .await
        .expect("failed to create ingest job");

        let job_id = created.id;
        let db_pool = self.db_pool.clone();

        tokio::spawn(async move {
            // Mark as running with started_at
            let _ = db::queries::IngestJobQueries::update_job_status(
                db_pool.pool(),
                job_id,
                JobStatus::Running,
                None,
                None,
            )
            .await;

            // Execute the loader
            let mut cmd = TokioCommand::new(loader_bin());
            cmd.arg("intelligent").arg(&url);
            if yes {
                cmd.arg("--yes");
            }
            cmd.env("DOC_TYPE_OVERRIDE", &doc_type);

            let result = run_cmd(cmd).await;

            match result {
                Ok(output) => {
                    let _ = db::queries::IngestJobQueries::update_job_status(
                        db_pool.pool(),
                        job_id,
                        JobStatus::Completed,
                        Some(&output),
                        None,
                    )
                    .await;
                }
                Err(e) => {
                    let _ = db::queries::IngestJobQueries::update_job_status(
                        db_pool.pool(),
                        job_id,
                        JobStatus::Failed,
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
                }
            }
        });

        job_id
    }

    /// Start a background cleanup task that prunes old jobs periodically
    pub fn start_cleanup_task(&self) {
        let db_pool = self.db_pool.clone();
        // Run every 5 minutes
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let _ = db::queries::IngestJobQueries::cleanup_old_jobs(db_pool.pool()).await;
            }
        });
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
            "status": job.status, // snake_case via sqlx Type derive
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
