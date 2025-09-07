use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Environment switch to use Redis-backed queue
#[must_use]
pub fn use_redis_queue() -> bool {
    std::env::var("USE_REDIS_QUEUE").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        || std::env::var("QUEUE_BACKEND").is_ok_and(|v| v.eq_ignore_ascii_case("redis"))
}

/// Resolve Redis URL from environment or use documented default
#[must_use]
pub fn redis_url_from_env() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| {
        // From background-job-system-requirements.md
        "redis://redis-auth-service.databases.svc.cluster.local:6379".to_string()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisJobMessage {
    pub job_id: Uuid,
    pub job_type: String,
    pub priority: i32,
    pub payload: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl RedisJobMessage {
    pub fn new(job_id: Uuid, job_type: impl Into<String>, priority: i32, payload: Value) -> Self {
        Self {
            job_id,
            job_type: job_type.into(),
            priority,
            payload,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Enqueue a job into Redis priority lists
///
/// Keys: `queue:<job_type>:p<priority>`
///
/// # Errors
/// Returns an error if Redis is not enabled or the enqueue operation fails.
pub async fn enqueue_job(msg: &RedisJobMessage) -> anyhow::Result<()> {
    use redis::AsyncCommands;

    if !use_redis_queue() {
        return Err(anyhow::anyhow!(
            "USE_REDIS_QUEUE not enabled; refusing to enqueue to Redis"
        ));
    }

    let url = redis_url_from_env();
    let client = redis::Client::open(url)?;
    let mut con = client.get_multiplexed_async_connection().await?;

    let key = format!("queue:{}:p{}", msg.job_type, msg.priority.max(1));
    let val = serde_json::to_string(msg)?;

    // LPUSH for FIFO across priorities when used with BRPOP in worker
    let _: i64 = con.lpush(key, val).await?;
    Ok(())
}
