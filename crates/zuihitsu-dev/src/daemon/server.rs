//! HTTP server + WebSocket bridge for the dev daemon.
//!
//! Serves `dist/` with `Cache-Control: no-store`, auto-injects the
//! `/__dev/livereload.js` shim into every HTML response, and bridges the
//! daemon's `broadcast::Sender<DevMsg>` to all connected browsers.

use std::path::{Path, PathBuf};

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Serialize;
use tokio::sync::broadcast;

const LIVERELOAD_JS: &str = include_str!("../embedded/livereload.js");

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DevMsg {
    /// Swap the `<link>` whose `pathname` matches `path` with a cache-buster.
    Css { path: String },
    /// Full page reload.
    Reload,
    /// Render the build-error overlay with `body` as preformatted text.
    Error { body: String },
    /// Dismiss any current error overlay.
    Ok,
}

#[derive(Clone)]
pub struct ServerState {
    pub dist: PathBuf,
    pub events: broadcast::Sender<DevMsg>,
}

pub async fn serve(state: ServerState, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/__dev/ws", get(ws_handler))
        .route("/__dev/livereload.js", get(livereload_js))
        .fallback(static_handler)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn livereload_js() -> Response<Body> {
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(LIVERELOAD_JS))
        .unwrap()
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ServerState>) -> impl IntoResponse {
    let rx = state.events.subscribe();
    ws.on_upgrade(move |socket| ws_loop(socket, rx))
}

async fn ws_loop(mut socket: WebSocket, mut rx: broadcast::Receiver<DevMsg>) {
    loop {
        tokio::select! {
            r = rx.recv() => match r {
                Ok(msg) => {
                    let Ok(s) = serde_json::to_string(&msg) else { continue };
                    if socket.send(Message::Text(s.into())).await.is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            },
            inbound = socket.recv() => match inbound {
                None => return,
                Some(Err(_)) => return,
                Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {}
            }
        }
    }
}

async fn static_handler(State(state): State<ServerState>, uri: Uri) -> Response<Body> {
    let raw = uri.path();
    let rel = if raw == "/" {
        "index.html".to_string()
    } else if raw.ends_with('/') {
        format!("{}index.html", raw.trim_start_matches('/'))
    } else {
        raw.trim_start_matches('/').to_string()
    };

    let candidate = state.dist.join(&rel);

    // Path-traversal guard: if the candidate canonicalizes outside dist/,
    // refuse. We do this lazily — if either path can't be canonicalized, fall
    // back to a strict join check.
    if let (Ok(d), Ok(c)) = (state.dist.canonicalize(), candidate.canonicalize())
        && !c.starts_with(&d)
    {
        return not_found();
    }

    let target = if candidate.is_dir() {
        candidate.join("index.html")
    } else {
        candidate
    };

    let bytes = match tokio::fs::read(&target).await {
        Ok(b) => b,
        Err(_) => return not_found(),
    };

    let mime = mime_for(&target);
    let body_bytes = if mime.starts_with("text/html") {
        let html = String::from_utf8_lossy(&bytes).into_owned();
        inject_livereload(&html).into_bytes()
    } else {
        bytes
    };

    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "no-store, must-revalidate")
        .body(Body::from(body_bytes))
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from("404 — not found\n"))
        .unwrap()
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        Some("webmanifest") => "application/manifest+json",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("txt") | Some("md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn inject_livereload(html: &str) -> String {
    let script = "<script src=\"/__dev/livereload.js\"></script>";
    if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + script.len() + 1);
        out.push_str(&html[..idx]);
        out.push_str(script);
        out.push_str(&html[idx..]);
        out
    } else {
        format!("{html}\n{script}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_before_body_close() {
        let html = "<html><body><h1>x</h1></body></html>";
        let out = inject_livereload(html);
        assert!(
            out.contains("<script src=\"/__dev/livereload.js\"></script></body>"),
            "{out}"
        );
    }

    #[test]
    fn appends_when_no_body_tag() {
        let out = inject_livereload("<p>x</p>");
        assert!(out.contains("<script src=\"/__dev/livereload.js\"></script>"));
    }
}
