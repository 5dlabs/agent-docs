//! Document loaders for various sources

#![allow(clippy::pedantic)] // Stub implementation - will be cleaned up when full scraping is added

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
// use std::collections::HashMap; // Not used in stub implementation
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};
use url::Url;

/// Rate limiter for docs.rs API calls
#[derive(Debug)]
pub struct RateLimiter {
    client: Client,
    last_request: Option<std::time::Instant>,
    min_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter (10 requests per minute = 6 seconds between requests)
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("doc-server-rust-loader/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            last_request: None,
            min_interval: Duration::from_secs(6), // 10 requests per minute
        }
    }

    /// Make a rate-limited HTTP GET request
    async fn get(&mut self, url: &str) -> Result<reqwest::Response> {
        // Enforce rate limiting
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                debug!(
                    "Rate limiting: waiting {:.2}s before next request",
                    wait_time.as_secs_f64()
                );
                time::sleep(wait_time).await;
            }
        }

        info!("Making HTTP request to: {}", url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        self.last_request = Some(std::time::Instant::now());

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP request failed with status: {}",
                response.status()
            ));
        }

        Ok(response)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Crate metadata from crates.io API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    pub name: String,
    pub newest_version: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
}

/// Documentation page extracted from docs.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPage {
    pub url: String,
    pub content: String,
    pub item_type: String, // "struct", "function", "module", etc.
    pub module_path: String,
    pub extracted_at: DateTime<Utc>,
}

/// Rust crate documentation loader with docs.rs integration
pub struct RustLoader {
    rate_limiter: RateLimiter,
}

impl Default for RustLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Many methods will be used when full scraping is implemented
impl RustLoader {
    /// Create a new Rust loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            rate_limiter: RateLimiter::new(),
        }
    }

    /// Load crate documentation from docs.rs and crates.io
    ///
    /// NOTE: This is a stub implementation for MVP to avoid Send/Sync issues with scraper
    ///
    /// # Errors
    ///
    /// Returns an error if the crate cannot be found or documentation cannot be fetched.
    pub async fn load_crate_docs(
        &mut self,
        crate_name: &str,
        version: Option<&str>,
    ) -> Result<(CrateMetadata, Vec<DocPage>)> {
        info!(
            "Loading documentation for crate: {} (version: {:?}) [STUB IMPLEMENTATION]",
            crate_name, version
        );

        // First, get crate metadata from crates.io
        let crate_metadata = self.fetch_crate_metadata(crate_name).await?;

        let target_version = version.unwrap_or(&crate_metadata.newest_version);
        info!(
            "Using version: {} for crate: {}",
            target_version, crate_name
        );

        // For MVP, create minimal stub documentation pages instead of scraping
        let doc_pages = self.create_stub_documentation(crate_name, target_version)?;

        info!(
            "Successfully created {} stub documentation pages for crate {}",
            doc_pages.len(),
            crate_name
        );
        Ok((crate_metadata, doc_pages))
    }

    /// Create stub documentation for MVP testing
    fn create_stub_documentation(&self, crate_name: &str, version: &str) -> Result<Vec<DocPage>> {
        let base_url = format!("https://docs.rs/{crate_name}/{version}");

        let stub_pages = vec![
            DocPage {
                url: base_url.clone(),
                content: format!(
                    "# {crate_name} v{version}\n\nThis is stub documentation for the {crate_name} crate version {version}.\n\nIn a full implementation, this would contain:\n- API documentation\n- Examples\n- Module descriptions\n- Function signatures\n\nThis stub allows testing the crate management infrastructure without scraping complexity."
                ),
                item_type: "crate".to_string(),
                module_path: crate_name.to_string(),
                extracted_at: Utc::now(),
            },
            DocPage {
                url: format!("{base_url}/struct.Example.html"),
                content: format!(
                    "# Example Struct\n\nA stub documentation page for an example struct in {crate_name}.\n\nThis demonstrates how individual API items would be stored as separate documents for better search granularity."
                ),
                item_type: "struct".to_string(),
                module_path: format!("{crate_name}::Example"),
                extracted_at: Utc::now(),
            }
        ];

        Ok(stub_pages)
    }

    /// Fetch crate metadata from crates.io API
    async fn fetch_crate_metadata(&mut self, crate_name: &str) -> Result<CrateMetadata> {
        let url = format!("https://crates.io/api/v1/crates/{crate_name}");
        let response = self.rate_limiter.get(&url).await?;
        let text = response.text().await?;

        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("Failed to parse crates.io response: {}", e))?;

        let crate_data = json
            .get("crate")
            .ok_or_else(|| anyhow!("Invalid crates.io response format"))?;

        Ok(CrateMetadata {
            name: crate_data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(crate_name)
                .to_string(),
            newest_version: crate_data
                .get("newest_version")
                .and_then(|v| v.as_str())
                .unwrap_or("latest")
                .to_string(),
            description: crate_data
                .get("description")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            documentation: crate_data
                .get("documentation")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
        })
    }

    /// Fetch documentation pages from docs.rs (unused in stub implementation)
    #[allow(dead_code)] // Will be used when full scraping is implemented
    async fn fetch_docs_rs_pages(
        &mut self,
        crate_name: &str,
        version: &str,
    ) -> Result<Vec<DocPage>> {
        let mut doc_pages = Vec::new();
        let base_url = format!("https://docs.rs/{crate_name}/{version}");

        // Start with the main crate page
        let main_page = self
            .fetch_single_page(&base_url, crate_name, "crate")
            .await?;
        doc_pages.push(main_page);

        // Try to find and fetch additional pages (modules, structs, etc.)
        let additional_pages = self
            .discover_documentation_pages(&base_url, crate_name)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to discover additional pages for {}: {}",
                    crate_name, e
                );
                Vec::new()
            });

        doc_pages.extend(additional_pages);

        Ok(doc_pages)
    }

    /// Fetch a single documentation page (unused in stub implementation)
    #[allow(dead_code)] // Will be used when full scraping is implemented
    async fn fetch_single_page(
        &mut self,
        url: &str,
        crate_name: &str,
        item_type: &str,
    ) -> Result<DocPage> {
        let response = self.rate_limiter.get(url).await?;
        let html_content = response.text().await?;

        // Parse HTML to extract main documentation content
        let document = Html::parse_document(&html_content);

        // Try different selectors to find the main content
        let content_selectors = [".docblock", ".content", ".rustdoc", "main"];

        let mut content = String::new();
        for selector_str in &content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    if !text.trim().is_empty() && text.len() > content.len() {
                        content = text;
                    }
                }
            }
        }

        if content.trim().is_empty() {
            // Fallback: use the entire text content
            content = document.root_element().text().collect::<Vec<_>>().join(" ");
        }

        // Clean up the content
        content = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Determine module path from URL
        let module_path = self.extract_module_path(url, crate_name);

        Ok(DocPage {
            url: url.to_string(),
            content,
            item_type: item_type.to_string(),
            module_path,
            extracted_at: Utc::now(),
        })
    }

    /// Discover additional documentation pages by parsing the main page
    async fn discover_documentation_pages(
        &mut self,
        base_url: &str,
        crate_name: &str,
    ) -> Result<Vec<DocPage>> {
        let response = self.rate_limiter.get(base_url).await?;
        let html_content = response.text().await?;
        let document = Html::parse_document(&html_content);

        let mut discovered_pages = Vec::new();
        let mut visited_urls = std::collections::HashSet::new();

        // Look for links to modules, structs, functions, etc.
        if let Ok(link_selector) = Selector::parse("a[href]") {
            for element in document.select(&link_selector).take(20) {
                // Limit to prevent too many requests
                if let Some(href) = element.value().attr("href") {
                    if let Some(full_url) = self.resolve_docs_rs_url(base_url, href) {
                        if visited_urls.insert(full_url.clone()) {
                            if let Ok(doc_page) =
                                self.fetch_docs_rs_item(&full_url, crate_name).await
                            {
                                discovered_pages.push(doc_page);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Discovered {} additional pages for crate: {}",
            discovered_pages.len(),
            crate_name
        );
        Ok(discovered_pages)
    }

    /// Resolve relative URLs for docs.rs links
    fn resolve_docs_rs_url(&self, base_url: &str, href: &str) -> Option<String> {
        if href.starts_with("http") {
            return Some(href.to_string());
        }

        if href.starts_with("../") || href.starts_with("./") || !href.starts_with('/') {
            if let Ok(base) = Url::parse(base_url) {
                if let Ok(resolved) = base.join(href) {
                    let url_str = resolved.to_string();
                    // Only include docs.rs URLs for the same crate
                    if url_str.contains("docs.rs") && !url_str.contains("#") {
                        return Some(url_str);
                    }
                }
            }
        }

        None
    }

    /// Fetch a specific docs.rs item (struct, function, module, etc.)
    async fn fetch_docs_rs_item(&mut self, url: &str, crate_name: &str) -> Result<DocPage> {
        let item_type = self.determine_item_type_from_url(url);
        self.fetch_single_page(url, crate_name, &item_type).await
    }

    /// Determine the type of documentation item from the URL
    fn determine_item_type_from_url(&self, url: &str) -> String {
        if url.contains("/struct.") {
            "struct".to_string()
        } else if url.contains("/fn.") {
            "function".to_string()
        } else if url.contains("/trait.") {
            "trait".to_string()
        } else if url.contains("/enum.") {
            "enum".to_string()
        } else if url.contains("/macro.") {
            "macro".to_string()
        } else {
            "module".to_string()
        }
    }

    /// Extract module path from documentation URL
    fn extract_module_path(&self, url: &str, crate_name: &str) -> String {
        // Parse the URL to extract the module path
        // Example: https://docs.rs/tokio/1.0.0/tokio/sync/index.html -> tokio::sync

        if let Ok(parsed_url) = Url::parse(url) {
            let path_segments: Vec<&str> = parsed_url
                .path_segments()
                .map(|c| c.collect::<Vec<_>>())
                .unwrap_or_default();

            // Find the crate name in the path and build module path from there
            if let Some(crate_index) = path_segments.iter().position(|&s| s == crate_name) {
                let module_parts: Vec<String> = path_segments
                    .iter()
                    .skip(crate_index)
                    .filter(|&&s| !s.is_empty() && s != "index.html")
                    .filter(|&s| !s.starts_with("struct.") && !s.starts_with("fn."))
                    .map(|s| s.replace(".html", ""))
                    .collect();

                if !module_parts.is_empty() {
                    return module_parts.join("::");
                }
            }
        }

        crate_name.to_string()
    }
}
