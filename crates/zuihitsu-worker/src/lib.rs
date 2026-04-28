//! zuihitsu Cloudflare Worker — Hashnode webhook receiver.
//!
//! Flow:
//!   Hashnode → POST /webhook (HMAC in `x-hashnode-signature`)
//!       → verify HMAC-SHA256 against WEBHOOK_SECRET binding
//!       → POST repository_dispatch to github.com/pleme-io/zuihitsu
//!       → GitHub Actions rebuilds dist/ and uploads via wrangler pages deploy
//!
//! Env bindings (set in wrangler.toml + secret store):
//!   - WEBHOOK_SECRET   — Hashnode-provided HMAC secret (prefixed whsec_)
//!   - GITHUB_TOKEN     — fine-grained PAT, repository_dispatch scope
//!   - GITHUB_REPO      — "pleme-io/zuihitsu"

use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use worker::{Context, Env, Request, Response, Result, Router, event};

type HmacSha256 = Hmac<Sha256>;

#[event(fetch, respond_with_errors)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    Router::new()
        .get("/healthz", |_, _| Response::ok("ok"))
        .post_async("/webhook", handle_webhook)
        .run(req, env)
        .await
}

#[derive(Debug, Deserialize)]
struct HashnodeEvent {
    #[serde(rename = "eventType")]
    event_type: String,
    #[serde(default)]
    post: Option<HashnodePost>,
}

#[derive(Debug, Deserialize)]
struct HashnodePost {
    slug: Option<String>,
    id: Option<String>,
}

async fn handle_webhook(mut req: Request, ctx: worker::RouteContext<()>) -> Result<Response> {
    let signature = req
        .headers()
        .get("x-hashnode-signature")?
        .ok_or_else(|| worker::Error::RustError("missing x-hashnode-signature".into()))?;

    let body_bytes = req.bytes().await?;

    let secret = ctx.secret("WEBHOOK_SECRET")?.to_string();
    if !verify_signature(&secret, &body_bytes, &signature) {
        return Response::error("invalid signature", 401);
    }

    let event: HashnodeEvent = serde_json::from_slice(&body_bytes)
        .map_err(|e| worker::Error::RustError(format!("bad json: {e}")))?;

    worker::console_log!(
        "hashnode event: {} slug={:?}",
        event.event_type,
        event.post.as_ref().and_then(|p| p.slug.clone())
    );

    let token = ctx.secret("GITHUB_TOKEN")?.to_string();
    let repo = ctx
        .var("GITHUB_REPO")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "pleme-io/zuihitsu".to_string());

    dispatch_rebuild(&token, &repo, &event).await?;

    Response::ok("ok")
}

fn verify_signature(secret: &str, body: &[u8], header: &str) -> bool {
    // Hashnode sends `sha256=<hex>` or just the hex digest depending on version;
    // accept both shapes.
    let hex_provided = header.strip_prefix("sha256=").unwrap_or(header);

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let Ok(provided) = hex::decode(hex_provided) else {
        return false;
    };
    // Constant-time compare.
    provided.len() == expected.len()
        && provided
            .iter()
            .zip(expected.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

async fn dispatch_rebuild(token: &str, repo: &str, event: &HashnodeEvent) -> Result<()> {
    use worker::{Fetch, Headers, Method, RequestInit};

    let url = format!("https://api.github.com/repos/{repo}/dispatches");

    let slug = event
        .post
        .as_ref()
        .and_then(|p| p.slug.clone())
        .unwrap_or_default();
    let body = json!({
        "event_type": "zuihitsu-rebuild",
        "client_payload": {
            "reason": event.event_type,
            "slug": slug,
        }
    });

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {token}"))?;
    headers.set("Accept", "application/vnd.github+json")?;
    headers.set("User-Agent", "zuihitsu-worker/0.1")?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.to_string().into()));

    let request = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() >= 300 {
        let text = resp.text().await.unwrap_or_default();
        return Err(worker::Error::RustError(format!(
            "github dispatch failed: {} {text}",
            resp.status_code()
        )));
    }
    worker::console_log!("github repository_dispatch sent for {repo}");
    Ok(())
}
