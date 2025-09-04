use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

use crate::server::McpServerState;

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

/// Run intelligent ingestion via the server-side loader binary.
///
/// # Errors
/// Returns an error response if validation fails or the loader command exits non-zero.
pub async fn intelligent_ingest_handler(
    State(_state): State<McpServerState>,
    Json(body): Json<IntelligentIngestRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.url.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "url is required".to_string()));
    }
    if body.doc_type.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "doc_type is required".to_string()));
    }

    // Build command: loader intelligent <url> --yes
    let mut cmd = TokioCommand::new(loader_bin());
    cmd.arg("intelligent").arg(&body.url);
    if body.yes {
        cmd.arg("--yes");
    }
    // Force doc_type via env for the intelligent ingestion
    cmd.env("DOC_TYPE_OVERRIDE", &body.doc_type);

    let output = run_cmd(cmd)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "status": "ok",
        "url": body.url,
        "doc_type": body.doc_type,
        "output": output,
    })))
}
