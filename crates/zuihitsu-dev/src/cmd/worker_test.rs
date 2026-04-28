//! `zuihitsu-dev worker-test` — sign + POST a fake Hashnode payload.
//!
//! Pairs with `wrangler dev --local` running the worker on :8787. Lets you
//! exercise the HMAC path, the github dispatch wiring, and the worker's
//! observability locally without configuring a real Hashnode webhook.

use anyhow::{Context, Result, anyhow};
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Slug to put in the mock payload's `post.slug`.
    #[arg(long, default_value = "test-slug")]
    pub slug: String,
    /// Event type to mimic.
    #[arg(long, default_value = "post.published")]
    pub event_type: String,
    /// Worker URL.
    #[arg(long, default_value = "http://127.0.0.1:8787/webhook")]
    pub url: String,
    /// HMAC secret (falls back to $WEBHOOK_SECRET).
    #[arg(long)]
    pub secret: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let secret = args
        .secret
        .or_else(|| std::env::var("WEBHOOK_SECRET").ok())
        .ok_or_else(|| anyhow!("--secret or $WEBHOOK_SECRET required"))?;

    let payload = serde_json::json!({
        "eventType": args.event_type,
        "post": { "slug": args.slug, "id": "mock" }
    });
    let body = serde_json::to_vec(&payload)?;

    let mut mac =
        <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).map_err(|e| anyhow!("hmac init: {e}"))?;
    mac.update(&body);
    let sig = hex::encode(mac.finalize().into_bytes());

    let client = reqwest::Client::new();
    let resp = client
        .post(&args.url)
        .header("x-hashnode-signature", format!("sha256={sig}"))
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .context("POST worker")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    println!("[worker-test] {status}: {text}");
    if !status.is_success() {
        anyhow::bail!("worker rejected payload (status {status})");
    }
    Ok(())
}
