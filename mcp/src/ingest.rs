use axum::{extract::Path, extract::State, http::StatusCode, Json};
use db::{models::JobStatus, DatabasePool};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::server::McpServerState;
use discovery::{IntelligentRepositoryAnalyzer, RepositoryAnalysis};
use std::fmt::Write as _;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct IntelligentIngestRequest {
    pub url: String,
    pub doc_type: String,
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
        |_| {
            // Try multiple possible locations for the loader binary
            let candidates = [
                "/app/loader",
                "./target/release/loader",
                "../target/release/loader",
                "target/release/loader",
            ];
            for candidate in &candidates {
                let path = std::path::PathBuf::from(candidate);
                if path.exists() {
                    return path;
                }
            }
            // Fallback to first candidate if none exist
            std::path::PathBuf::from("/app/loader")
        },
        std::path::PathBuf::from,
    )
}

fn ingest_debug_enabled() -> bool {
    std::env::var("INGEST_DEBUG").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
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
        db::queries::IngestJobQueries::find_job_by_id(self.db_pool.pool(), id)
            .await
            .unwrap_or_default()
    }

    /// Enqueue a new intelligent ingest job in the database and spawn processing.
    ///
    /// # Errors
    /// Returns an error if the job record cannot be created in the database.
    pub async fn enqueue(&self, url: String, doc_type: String) -> anyhow::Result<Uuid> {
        // Create job in DB first so any replica can see it
        let created =
            db::queries::IngestJobQueries::create_job(self.db_pool.pool(), &url, &doc_type).await?;

        let job_id = created.id;
        let db_pool = self.db_pool.clone();

        tokio::spawn(async move {
            info!(%job_id, %url, %doc_type, "Starting intelligent ingest job");
            let _ = db::queries::IngestJobQueries::update_job_status(
                db_pool.pool(),
                job_id,
                JobStatus::Running,
                None,
                None,
            )
            .await;

            // 1) Run discovery (Claude Code) to get a plan
            let mut analyzer = IntelligentRepositoryAnalyzer::new();

            let analysis = match analyzer.analyze_repository(&url).await {
                Ok(a) => a,
                Err(e) => {
                    warn!(%job_id, err = %e, "Discovery failed");
                    let _ = db::queries::IngestJobQueries::update_job_status(
                        db_pool.pool(),
                        job_id,
                        JobStatus::Failed,
                        None,
                        Some(&e.to_string()),
                    )
                    .await;
                    return;
                }
            };

            // 2) Execute plan with strict allowlist
            let exec_res = execute_cli_plan(&analysis, &doc_type).await;
            match exec_res {
                Ok(output) => {
                    debug!(%job_id, out_len = output.len(), "Ingest completed");
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
                    warn!(%job_id, err = %e, "Ingest plan execution failed");
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

        Ok(job_id)
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

fn work_base() -> std::path::PathBuf {
    std::env::var("INGEST_WORK_DIR").map_or_else(|_| std::env::temp_dir(), std::path::PathBuf::from)
}

/// Execute discovery CLI commands with a strict allowlist
async fn execute_cli_plan(analysis: &RepositoryAnalysis, doc_type: &str) -> anyhow::Result<String> {
    let mut combined = String::new();

    for (i, original_cmd) in analysis.cli_commands.iter().enumerate() {
        // Remap /tmp to work_base
        let cmd = if original_cmd.contains("/tmp/") {
            original_cmd.replace("/tmp", work_base().to_string_lossy().as_ref())
        } else {
            original_cmd.clone()
        };

        // Normalize cargo/loader invocations
        let (program, args) = normalize_command(&cmd, doc_type);
        // Allowlist check
        ensure_allowed(&program, &args)?;

        let mut command = TokioCommand::new(&program);
        if ingest_debug_enabled() {
            command.env(
                "RUST_LOG",
                std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| "debug,loader=debug,mcp=debug".to_string()),
            );
        }
        command.args(args);
        let out = run_cmd(command).await?;
        write!(&mut combined, "\n# Step {} output:\n{}\n", i + 1, out)?;
    }

    Ok(combined)
}

fn normalize_command(cmd: &str, doc_type: &str) -> (String, Vec<String>) {
    let loader = loader_bin().to_string_lossy().to_string();
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return (loader, vec![]);
    }

    // Handle cargo run --bin loader commands (more flexible pattern matching)
    if parts[0] == "cargo" && parts.get(1) == Some(&"run") {
        // Look for --bin loader or just --bin followed by loader
        let mut found_loader_bin = false;
        let mut args_start = parts.len();

        for (i, &part) in parts.iter().enumerate() {
            if part == "--bin" {
                if let Some(&next) = parts.get(i + 1) {
                    if next == "loader" {
                        found_loader_bin = true;
                        // Find the next -- which should indicate the start of loader arguments
                        for j in (i + 2)..parts.len() {
                            if parts[j] == "--" {
                                args_start = j + 1;
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }

        if found_loader_bin {
            let mut args: Vec<String> = parts[args_start..].iter().map(|&s| s.to_owned()).collect();
            enforce_doc_type(&mut args, doc_type);
            return (loader, args);
        }
    }

    // Handle direct loader commands
    if parts[0] == "loader" {
        let mut args: Vec<String> = parts[1..].iter().map(|&s| s.to_owned()).collect();
        enforce_doc_type(&mut args, doc_type);
        return (loader, args);
    }

    // Handle git commands
    if parts[0] == "git" {
        return (
            "git".to_string(),
            parts[1..].iter().map(|&s| s.to_owned()).collect(),
        );
    }

    // Fallback: return as-is (will be rejected by allowlist)
    (
        parts[0].to_string(),
        parts[1..].iter().map(|&s| s.to_owned()).collect(),
    )
}

fn enforce_doc_type(args: &mut Vec<String>, doc_type: &str) {
    if args.first().map(String::as_str) == Some("database") {
        // ensure --doc-type doc_type present (override if needed)
        let mut out = Vec::with_capacity(args.len() + 2);
        let mut i = 0;
        let mut set = false;
        while i < args.len() {
            if args[i] == "--doc-type" {
                out.push("--doc-type".to_string());
                if i + 1 < args.len() {
                    i += 1;
                } // skip value
                out.push(doc_type.to_string());
                set = true;
            } else {
                out.push(args[i].clone());
            }
            i += 1;
        }
        if !set {
            out.push("--doc-type".to_string());
            out.push(doc_type.to_string());
        }
        *args = out;
    }
}

fn ensure_allowed(program: &str, args: &[String]) -> anyhow::Result<()> {
    match program {
        p if p == loader_bin().to_string_lossy() => {
            if args.is_empty() {
                return Err(anyhow::anyhow!("missing loader subcommand"));
            }
            match args[0].as_str() {
                "cli" | "database" => Ok(()),
                other => Err(anyhow::anyhow!(format!(
                    "loader subcommand not allowed: {}",
                    other
                ))),
            }
        }
        "git" => {
            // allow: git clone --depth 1 <url> <dest>
            if args.first().map(String::as_str) != Some("clone") {
                return Err(anyhow::anyhow!("only 'git clone' is allowed"));
            }
            if !args
                .iter()
                .any(|a| a == "--depth" || a.starts_with("--depth"))
            {
                return Err(anyhow::anyhow!("git clone must include --depth"));
            }
            if let Some(dest) = args.last() {
                let base = work_base();
                let dest_path = std::path::Path::new(dest);
                if !dest_path.starts_with(&base) {
                    return Err(anyhow::anyhow!(
                        "git clone destination is outside work base"
                    ));
                }
            }
            Ok(())
        }
        _ => Err(anyhow::anyhow!(format!("program not allowed: {}", program))),
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
        .enqueue(body.url, body.doc_type)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create job: {e}"),
            )
        })?;

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
