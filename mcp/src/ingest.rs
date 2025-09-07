use axum::{extract::Path, extract::State, http::StatusCode, Json};
use db::{models::JobStatus, DatabasePool};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::server::McpServerState;
use discovery::{IntelligentRepositoryAnalyzer, RepositoryAnalysis};
use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};
use tokio::sync::{oneshot, Semaphore};
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
            // Global concurrency cap for ingest jobs
            let _permit = get_ingest_semaphore().acquire_owned().await.ok();
            info!(%job_id, %url, %doc_type, "Starting intelligent ingest job");
            let _ = db::queries::IngestJobQueries::update_job_status(
                db_pool.pool(),
                job_id,
                JobStatus::Running,
                None,
                None,
            )
            .await;

            // Heartbeat task to keep updated_at fresh while job runs
            let (hb_tx, mut hb_rx) = oneshot::channel::<()>();
            let hb_pool = db_pool.clone();
            let hb_job_id = job_id;
            let hb_handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            let _ = db::queries::IngestJobQueries::update_job_status(
                                hb_pool.pool(),
                                hb_job_id,
                                JobStatus::Running,
                                None,
                                None,
                            ).await;
                        }
                        _ = &mut hb_rx => {
                            break;
                        }
                    }
                }
            });

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
                    // Stop heartbeat
                    let _ = hb_tx.send(());
                    let _ = hb_handle.await;
                    return;
                }
            };

            // 2) Execute plan with strict allowlist
            let exec_res = execute_cli_plan(&analysis, &doc_type, &url).await;
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

            // Stop heartbeat
            let _ = hb_tx.send(());
            let _ = hb_handle.await;
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

// Global semaphore for ingest concurrency
fn ingest_max_concurrency() -> usize {
    std::env::var("INGEST_MAX_CONCURRENCY")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(2)
}

static INGEST_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn get_ingest_semaphore() -> Arc<Semaphore> {
    INGEST_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(ingest_max_concurrency())))
        .clone()
}

fn work_base() -> std::path::PathBuf {
    std::env::var("INGEST_WORK_DIR").map_or_else(|_| std::env::temp_dir(), std::path::PathBuf::from)
}

/// Generate a unique directory name for repository analysis based on repo URL and timestamp
fn generate_unique_repo_dir(repo_url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut hasher = DefaultHasher::new();
    repo_url.hash(&mut hasher);
    let hash = hasher.finish();

    // Create a unique directory name combining timestamp and hash
    format!("repo-analysis-{}-{}", timestamp, hash % 10000)
}

/// Execute discovery CLI commands with a strict allowlist
#[allow(clippy::too_many_lines)]
async fn execute_cli_plan(
    analysis: &RepositoryAnalysis,
    doc_type: &str,
    repo_url: &str,
) -> anyhow::Result<String> {
    // Generate unique directory for this ingestion
    let unique_repo_dir = generate_unique_repo_dir(repo_url);
    let unique_docs_dir = format!("{}_out", unique_repo_dir);

    let mut combined = String::new();
    let mut executed_cli_steps: usize = 0;
    let strict_plan = std::env::var("INGEST_STRICT_PLAN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    for (i, original_cmd) in analysis.cli_commands.iter().enumerate() {
        // Remap placeholders and /tmp to work_base
        let cmd = original_cmd
            .replace(
                "UNIQUE_REPO_DIR",
                &work_base().join(&unique_repo_dir).to_string_lossy(),
            )
            .replace(
                "UNIQUE_DOCS_OUT",
                &work_base().join(&unique_docs_dir).to_string_lossy(),
            )
            .replace("/tmp", work_base().to_string_lossy().as_ref());

        // Normalize cargo/loader invocations
        let (program, mut args) = normalize_command(&cmd, doc_type);
        // Allowlist check
        ensure_allowed(&program, &args)?;

        // If this is a loader cli invocation, ensure the include path exists; otherwise skip or fallback
        if program == loader_bin().to_string_lossy() && !args.is_empty() && args[0] == "cli" {
            // Expect path argument right after subcommand
            if args.len() > 1 {
                let requested_path = std::path::PathBuf::from(&args[1]);
                if !requested_path.exists() {
                    if strict_plan {
                        let msg = format!(
                            "Requested path not found ({}). Strict plan enabled; not applying fallbacks.",
                            requested_path.display()
                        );
                        warn!("{}", msg);
                        let _ = writeln!(combined, "⚠️  {msg}");
                        continue;
                    }
                    // Attempt a smart fallback: check common documentation directories inside the cloned repo
                    let unique_repo_dir = generate_unique_repo_dir(repo_url);
                    let repo_root = work_base().join(&unique_repo_dir);
                    let candidates = [
                        repo_root.join("docs"),
                        repo_root.join("Documentation"),
                        repo_root.join("website/docs"),
                        repo_root.join("website/content"),
                        repo_root.join("website/content/docs"),
                        repo_root.join("docs/website"),
                        repo_root.join("content/docs"),
                        repo_root.join("docs/content"),
                        repo_root.join("doc"),
                        repo_root.join("docs-src"),
                        repo_root.join("guides"),
                        repo_root.join("site/docs"),
                    ];

                    let fallback = candidates.iter().find(|p| p.exists());
                    if let Some(found) = fallback {
                        let msg = format!(
                            "Requested path not found ({}). Falling back to {}",
                            requested_path.display(),
                            found.display()
                        );
                        warn!("{}", msg);
                        let _ = writeln!(combined, "⚠️  {msg}");
                        // Replace the include path with the fallback
                        args[1] = found.to_string_lossy().to_string();
                    } else {
                        let msg = format!(
                            "Requested path not found ({}). No suitable fallback found; skipping this step.",
                            requested_path.display()
                        );
                        warn!("{}", msg);
                        let _ = writeln!(combined, "⚠️  {msg}");
                        // Skip this command and continue with the plan
                        continue;
                    }
                }
                // Respect plan's extensions by default; optionally augment via env
                let augment_extensions_enabled = std::env::var("INGEST_AUGMENT_EXTENSIONS")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                // Normalize/augment --extensions
                let mut i = 1usize;
                let mut found_ext = false;
                while i + 1 < args.len() {
                    if args[i] == "--extensions" {
                        found_ext = true;
                        if augment_extensions_enabled {
                            if let Some(exts) = args.get(i + 1).cloned() {
                                let mut set: std::collections::BTreeSet<String> = exts
                                    .split(',')
                                    .map(|s| s.trim().to_lowercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                for e in [
                                    "md", "mdx", "rst", "html", "json", "yaml", "yml", "toml",
                                    "txt",
                                ] {
                                    set.insert(e.to_string());
                                }
                                args[i + 1] = set.into_iter().collect::<Vec<_>>().join(",");
                            }
                        }
                        break;
                    }
                    i += 1;
                }
                if !found_ext && !strict_plan {
                    args.push("--extensions".to_string());
                    // Minimal focused default when model omitted
                    args.push("md,mdx,rst,html,txt".to_string());
                }
                // Ensure --recursive present
                if !strict_plan && !args.iter().any(|a| a == "--recursive") {
                    args.push("--recursive".to_string());
                }
            }
        }

        let mut command = TokioCommand::new(&program);
        if ingest_debug_enabled() {
            command.env(
                "RUST_LOG",
                std::env::var("RUST_LOG")
                    .unwrap_or_else(|_| "debug,loader=debug,mcp=debug".to_string()),
            );
        }
        let args_clone = args.clone();
        command.args(args_clone);
        let out = run_cmd(command).await?;
        write!(&mut combined, "\n# Step {} output:\n{}\n", i + 1, out)?;
        if program == loader_bin().to_string_lossy() && !args.is_empty() && args[0] == "cli" {
            executed_cli_steps += 1;
        }
    }

    // If none of the CLI steps were executed (e.g., all paths invalid), attempt auto-detection
    if executed_cli_steps == 0 && !strict_plan {
        let unique_repo_dir = generate_unique_repo_dir(repo_url);
        let repo_root = work_base().join(&unique_repo_dir);
        let candidates = [
            repo_root.join("docs"),
            repo_root.join("Documentation"),
            repo_root.join("website/content/docs"),
            repo_root.join("website/docs"),
            repo_root.join("docs/source"),
            repo_root.join("content/docs"),
            repo_root.join("docs/content"),
            repo_root.join("doc"),
            repo_root.join("README.md"),
            repo_root.join("README.MD"),
        ];
        let mut found_any = false;
        for dir in candidates.iter().filter(|p| p.exists()) {
            let mut command = TokioCommand::new(loader_bin());
            let args = vec![
                "cli".to_string(),
                dir.to_string_lossy().to_string(),
                "--extensions".to_string(),
                "md,mdx,rst,html,json,yaml,yml,toml,txt".to_string(),
                "--recursive".to_string(),
                "-o".to_string(),
                work_base().join("docs_out").to_string_lossy().to_string(),
            ];
            ensure_allowed(&loader_bin().to_string_lossy(), &args)?;
            command.args(&args);
            let out = run_cmd(command).await?;
            found_any = true;
            write!(
                &mut combined,
                "\n# Auto-detected CLI step for {}:\n{}\n",
                dir.display(),
                out
            )?;
        }
        if !found_any {
            let _ = writeln!(
                combined,
                "⚠️  No valid documentation directories detected under {}",
                repo_root.display()
            );
        }
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
                        if let Some((idx, _)) = parts
                            .iter()
                            .enumerate()
                            .skip(i + 2)
                            .find(|(_, &part)| part == "--")
                        {
                            args_start = idx + 1;
                        }
                        break;
                    }
                }
            }
        }

        if found_loader_bin {
            let mut args: Vec<String> = parts[args_start..].iter().map(|&s| s.to_owned()).collect();
            enforce_doc_type(&mut args, doc_type);
            sanitize_loader_args(&mut args);
            return (loader, args);
        }
    }

    // Handle direct loader commands
    if parts[0] == "loader" {
        let mut args: Vec<String> = parts[1..].iter().map(|&s| s.to_owned()).collect();
        enforce_doc_type(&mut args, doc_type);
        sanitize_loader_args(&mut args);
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

/// Remove or translate flags the loader CLI does not support.
fn sanitize_loader_args(args: &mut Vec<String>) {
    if args.is_empty() {
        return;
    }
    if args[0] != "cli" {
        return;
    }
    // Remove flags we don't support and their values (if present)
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--exclude-patterns" || a == "--include-patterns" {
            // drop this flag and consume its value if present and not another flag
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 1; // skip value
            }
        } else if a == "--output" {
            // translate to -o
            out.push("-o".to_string());
        } else {
            out.push(a.clone());
        }
        i += 1;
    }
    *args = out;
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
