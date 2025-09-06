use anyhow::{anyhow, Result};
use std::process::Stdio;
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
            .env("DISABLE_AUTOUPDATER", "1");
        if std::env::var("CLAUDE_CONFIG_DIR").is_err() {
            cmd.env("CLAUDE_CONFIG_DIR", "/tmp/claude-code-config");
        }

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
