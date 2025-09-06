//! Rust crate ingestion: fetch crate metadata and docs (stub docs.rs scraping).

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
// (no serde_json::Value import)
use std::time::Duration;
use tokio::time;
use tracing::{debug, info};
use url::Url;

#[derive(Debug)]
pub struct RateLimiter {
    client: Client,
    last_request: Option<std::time::Instant>,
    min_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Panics
    /// Panics if the HTTP client cannot be created.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("doc-server-rust-loader/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            last_request: None,
            min_interval: Duration::from_secs(6),
        }
    }

    /// Perform a rate-limited GET request.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response status is not successful.
    pub async fn get(&mut self, url: &str) -> Result<reqwest::Response> {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                debug!("Rate limiting: waiting {:.2}s", wait_time.as_secs_f64());
                time::sleep(wait_time).await;
            }
        }
        info!("HTTP GET: {}", url);
        let resp = self.client.get(url).send().await.map_err(|e| anyhow!("HTTP failed: {}", e))?;
        self.last_request = Some(std::time::Instant::now());
        if !resp.status().is_success() {
            return Err(anyhow!("HTTP status: {}", resp.status()));
        }
        Ok(resp)
    }
}

impl Default for RateLimiter { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    pub name: String,
    pub newest_version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPage {
    pub url: String,
    pub content: String,
    pub item_type: String,
    pub module_path: String,
    pub extracted_at: DateTime<Utc>,
}

pub struct RustLoader { rate_limiter: RateLimiter }
impl Default for RustLoader { fn default() -> Self { Self::new() } }

impl RustLoader {
    #[must_use]
    pub fn new() -> Self { Self { rate_limiter: RateLimiter::new() } }

    /// Load crate metadata and documentation pages.
    ///
    /// # Errors
    /// Returns an error if fetching metadata or pages fails.
    pub async fn load_crate_docs(&mut self, crate_name: &str, version: Option<&str>) -> Result<(CrateMetadata, Vec<DocPage>)> {
        info!("Loading crate docs: {} (version: {:?}) [stub]", crate_name, version);
        let meta = self.fetch_crate_metadata(crate_name).await?;
        let target = version.unwrap_or(&meta.newest_version);
        let pages = Self::create_stub_documentation(crate_name, target);
        Ok((meta, pages))
    }

    fn create_stub_documentation(crate_name: &str, version: &str) -> Vec<DocPage> {
        let base_url = format!("https://docs.rs/{crate_name}/{version}");
        vec![
            DocPage {
                url: base_url.clone(),
                content: format!("# {crate_name} v{version}\n\nStub documentation page."),
                item_type: "crate".into(),
                module_path: crate_name.into(),
                extracted_at: Utc::now(),
            },
            DocPage {
                url: format!("{base_url}/struct.Example.html"),
                content: "# Example Struct\n\nStub page for Example struct.".into(),
                item_type: "struct".into(),
                module_path: format!("{crate_name}::Example"),
                extracted_at: Utc::now(),
            },
        ]
    }

    async fn fetch_crate_metadata(&mut self, crate_name: &str) -> Result<CrateMetadata> {
        let url = format!("https://crates.io/api/v1/crates/{crate_name}");
        let text = self.get_text(&url).await?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| anyhow!("Parse crates.io: {}", e))?;
        let c = json.get("crate").ok_or_else(|| anyhow!("Invalid crates.io response"))?;
        Ok(CrateMetadata {
            name: c.get("id").and_then(|v| v.as_str()).unwrap_or(crate_name).to_string(),
            newest_version: c.get("newest_version").and_then(|v| v.as_str()).unwrap_or("latest").to_string(),
            description: c.get("description").and_then(|v| v.as_str()).map(ToString::to_string),
            documentation: c.get("documentation").and_then(|v| v.as_str()).map(ToString::to_string),
        })
    }

    async fn get_text(&mut self, url: &str) -> Result<String> {
        let resp = self.rate_limiter.get(url).await?;
        Ok(resp.text().await?)
    }

    #[allow(dead_code)]
    async fn fetch_single_page(&mut self, url: &str, crate_name: &str, item_type: &str) -> Result<DocPage> {
        let text = self.get_text(url).await?;
        let document = Html::parse_document(&text);
        let content = Selector::parse("body").ok()
            .map_or_else(
                || document.root_element().text().collect::<Vec<_>>().join(" "),
                |sel| {
                    let mut best = String::new();
                    for el in document.select(&sel) {
                        let t = el.text().collect::<Vec<_>>().join(" ");
                        if t.len() > best.len() { best = t; }
                    }
                    best
                },
            );
        let module_path = Self::extract_module_path(url, crate_name);
        Ok(DocPage { url: url.into(), content, item_type: item_type.into(), module_path, extracted_at: Utc::now() })
    }

    fn extract_module_path(url: &str, crate_name: &str) -> String {
        if let Ok(parsed) = Url::parse(url) {
            let parts: Vec<&str> = parsed
                .path_segments()
                .map(std::iter::Iterator::collect)
                .unwrap_or_default();
            if let Some(idx) = parts.iter().position(|&s| s == crate_name) {
                let module: Vec<String> = parts.iter().skip(idx)
                    .filter(|&&s| !s.is_empty() && s != "index.html")
                    .filter(|&s| !s.starts_with("struct.") && !s.starts_with("fn."))
                    .map(|s| s.replace(".html", ""))
                    .collect();
                if !module.is_empty() { return module.join("::"); }
            }
        }
        crate_name.to_string()
    }
}
