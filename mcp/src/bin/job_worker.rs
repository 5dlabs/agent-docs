
use anyhow::Result;
use db::{models::JobStatus, DatabasePool};
use mcp::queue::{redis_url_from_env, RedisJobMessage};
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("Starting Redis job worker...");

    let db_pool = DatabasePool::from_env().await?;

    let url = redis_url_from_env();
    let client = redis::Client::open(url)?;
    let mut con = client.get_multiplexed_async_connection().await?;

    // Job types to process
    let job_types: Vec<String> = std::env::var("WORKER_JOB_TYPES")
        .unwrap_or_else(|_| "ingest,crate_add".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Priorities: 5..1 (5 = highest)
    let priorities: Vec<i32> = vec![5, 4, 3, 2, 1];

    loop {
        // Build queue keys by priority then job_type
        let mut keys: Vec<String> = Vec::new();
        for p in &priorities {
            for jt in &job_types {
                keys.push(format!("queue:{jt}:p{p}"));
            }
        }

        // Block for next job
        let res: Option<(String, String)> = match redis::cmd("BRPOP")
            .arg(&keys)
            .arg(0)
            .query_async(&mut con)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!("Redis BRPOP error: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        let Some((_key, payload)) = res else { continue };
        let msg: RedisJobMessage = match serde_json::from_str(&payload) {
            Ok(m) => m,
            Err(e) => {
                warn!("Invalid job payload, skipping: {e}");
                continue;
            }
        };

        match msg.job_type.as_str() {
            "ingest" => {
                if let Err(e) = handle_ingest(&db_pool, &msg.payload, msg.job_id).await {
                    warn!("Ingest job failed: {e}");
                }
            }
            "crate_add" => {
                if let Err(e) = handle_crate_add(&db_pool, &msg.payload, msg.job_id).await {
                    warn!("Crate add job failed: {e}");
                }
            }
            other => {
                warn!("Unknown job type '{other}', ignoring");
            }
        }
    }
}

#[derive(Deserialize)]
struct IngestPayload {
    url: String,
    doc_type: String,
}

async fn handle_ingest(db_pool: &DatabasePool, payload: &Value, job_id: uuid::Uuid) -> Result<()> {
    use discovery::IntelligentRepositoryAnalyzer;
    use mcp::ingest::execute_cli_plan;

    let p: IngestPayload = serde_json::from_value(payload.clone())?;

    let _ = db::queries::IngestJobQueries::update_job_status(
        db_pool.pool(),
        job_id,
        JobStatus::Running,
        None,
        None,
    )
    .await?;

    let mut analyzer = IntelligentRepositoryAnalyzer::new();
    match analyzer.analyze_repository(&p.url).await {
        Ok(analysis) => match execute_cli_plan(&analysis, &p.doc_type, &p.url).await {
            Ok(output) => {
                let _ = db::queries::IngestJobQueries::update_job_status(
                    db_pool.pool(),
                    job_id,
                    JobStatus::Completed,
                    Some(&output),
                    None,
                )
                .await?;
            }
            Err(e) => {
                let _ = db::queries::IngestJobQueries::update_job_status(
                    db_pool.pool(),
                    job_id,
                    JobStatus::Failed,
                    None,
                    Some(&e.to_string()),
                )
                .await?;
            }
        },
        Err(e) => {
            let _ = db::queries::IngestJobQueries::update_job_status(
                db_pool.pool(),
                job_id,
                JobStatus::Failed,
                None,
                Some(&e.to_string()),
            )
            .await?;
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct CrateAddPayload {
    crate_name: String,
    version: Option<String>,
    features: Option<Vec<String>>,
    include_dev_deps: bool,
    force_update: bool,
    atomic_rollback: bool,
}

async fn handle_crate_add(db_pool: &DatabasePool, payload: &Value, job_id: uuid::Uuid) -> Result<()> {
    use embed::client::EmbeddingClient;
    use embed::OpenAIEmbeddingClient;
    use mcp::crate_tools::AddRustCrateTool;
    use rust_crates::RustLoader;
    use std::sync::Arc as StdArc;

    let p: CrateAddPayload = serde_json::from_value(payload.clone())?;

    let client: StdArc<dyn EmbeddingClient + Send + Sync> = StdArc::new(OpenAIEmbeddingClient::new()?);
    let tool = AddRustCrateTool::new(db_pool.clone(), client.clone());

    // Construct a minimal call path by invoking the internal ingestion function
    // Note: We replicate the process similar to the in-server background task
    let mut loader = RustLoader::new();
    let processor = mcp::job_queue::CrateJobProcessor::new(db_pool.clone());

    let _ = processor
        .update_job_status(job_id, JobStatus::Running, Some(0), None)
        .await?;

    // We call the same internal processing method via a public facade exposed by the tool
    tool.process_in_worker(
        &processor,
        &mut loader,
        &client,
        db_pool,
        job_id,
        &p.crate_name,
        p.version.as_deref(),
        p.features.as_ref(),
        p.include_dev_deps,
        p.force_update,
        p.atomic_rollback,
    )
    .await
}
