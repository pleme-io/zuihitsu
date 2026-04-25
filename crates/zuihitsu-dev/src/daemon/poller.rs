//! Background Hashnode poller.
//!
//! Every `interval`, list summaries; if the response hash changed since the
//! previous poll, invalidate the disk cache and trigger a sitegen + reload.
//! Lets the browser auto-update when a post is published or edited on
//! Hashnode without operator intervention.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::UnboundedSender;

use super::build::BuildRequest;

const POLL_QUERY: &str = "query Poll($host: String!) { publication(host: $host) { posts(first: 50) { edges { node { id slug title publishedAt updatedAt } } } } }";

pub async fn run(interval: Duration, cache_dir: PathBuf, build_tx: UnboundedSender<BuildRequest>) {
    let host = std::env::var("ZUIHITSU_HASHNODE_HOST")
        .unwrap_or_else(|_| "drzln.hashnode.dev".to_string());
    let body = json!({ "query": POLL_QUERY, "variables": { "host": host } });
    let client = match reqwest::Client::builder()
        .user_agent("zuihitsu-dev/0.1 poller")
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "poller: client init failed");
            return;
        }
    };

    let mut prev: Option<String> = None;
    loop {
        tokio::time::sleep(interval).await;
        let raw = match client
            .post("https://gql.hashnode.com/")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => {
                tracing::debug!(error = %e, "poll: hashnode fetch failed");
                continue;
            }
        };
        let mut h = Sha256::new();
        h.update(raw.as_bytes());
        let hash = hex::encode(h.finalize());

        if let Some(prev_h) = &prev
            && *prev_h != hash
        {
            tracing::info!("poll: hashnode content changed → invalidating cache");
            let _ = std::fs::remove_dir_all(&cache_dir);
            let _ = std::fs::create_dir_all(&cache_dir);
            let _ = build_tx.send(BuildRequest::SitegenOnly {
                only: vec!["all".into()],
            });
        }
        prev = Some(hash);
    }
}
