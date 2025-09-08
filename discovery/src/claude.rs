use anyhow::{anyhow, Result};
use std::process::Stdio;
use std::{fs, path::{Path, PathBuf}};

// Ensure a directory exists and is writable by probing file creation.
fn ensure_writable_dir(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".perm_test");
    match fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::debug;

/// Minimal Claude Code runner for stream-json prompts
pub struct ClaudeRunner {
    pub binary_path: String,
    pub model_name: String,
}

impl ClaudeRunner {
    #[must_use]
    pub fn new() -> Self {
        let binary = std::env::var("CLAUDE_BINARY_PATH").unwrap_or_else(|_| "claude".to_string());
        let model = std::env::var("CLAUDE_MODEL")
            .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());
        Self {
            binary_path: binary,
            model_name: model,
        }
    }

    /// Execute a single user prompt and return stdout
    #[allow(clippy::too_many_lines)]
    pub async fn run(&self, prompt: &str) -> Result<String> {
        let timeout_secs: u64 = std::env::var("CLAUDE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);

        let json_message = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(prompt)?
        );

        // Resolve a writable config directory for the Claude CLI.
        // Prefer explicitly provided CLAUDE_CONFIG_DIR if writable; otherwise
        // fall back to a per-run directory under INGEST_WORK_DIR.
        let configured_dir = std::env::var("CLAUDE_CONFIG_DIR").ok();
        let ingest_dir = std::env::var("INGEST_WORK_DIR").unwrap_or_else(|_| "/tmp".into());

        // Try the configured dir first if present and writable
        let selected_config_dir: PathBuf = if let Some(cfg) = configured_dir {
            let p = PathBuf::from(cfg);
            if ensure_writable_dir(&p) {
                p
            } else {
                // Fallback: unique per-run directory under ingest dir
                let mut p = PathBuf::from(&ingest_dir);
                // Include a lightweight unique suffix (secs + pid)
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let unique = format!("claude-code-config-{}-{}", secs, std::process::id());
                p.push(unique);
                let _ = ensure_writable_dir(&p);
                p
            }
        } else {
            // Default path if none configured: use ingest dir
            let mut p = PathBuf::from(&ingest_dir);
            p.push("claude-code-config");
            // If shared dir is not writable, create a unique subdir
            if !ensure_writable_dir(&p) {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let unique = format!("claude-code-config-{}-{}", secs, std::process::id());
                p.pop();
                p.push(unique);
                let _ = ensure_writable_dir(&p);
            }
            p
        };

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--input-format")
            .arg("stream-json")
            .arg("--verbose")
            .env("CLAUDE_MODEL", &self.model_name)
            .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
            .env("DISABLE_TELEMETRY", "1")
            .env("DISABLE_ERROR_REPORTING", "1")
            .env("DISABLE_AUTOUPDATER", "1")
            .env("CLAUDE_CONFIG_DIR", selected_config_dir.as_os_str());

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                anyhow!(
                    "Failed to start Claude binary '{}': {}",
                    self.binary_path,
                    e
                )
            })?;

        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        stdin.write_all(json_message.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);

        let out = timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await;
        match out {
            Ok(Ok(output)) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow!(
                        "Claude binary exited with status {}: {}",
                        output.status,
                        stderr.chars().take(1000).collect::<String>()
                    ));
                }
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                debug!(len = stdout.len(), "Claude stdout received");
                if stdout.trim().is_empty() {
                    return Err(anyhow!("Claude returned empty response"));
                }
                Ok(stdout.trim().to_string())
            }
            Ok(Err(e)) => Err(anyhow!("Failed to read Claude output: {}", e)),
            Err(_) => Err(anyhow!("Claude timed out after {}s", timeout_secs)),
        }
    }
}
