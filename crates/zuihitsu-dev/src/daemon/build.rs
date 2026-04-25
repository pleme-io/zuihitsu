//! Cargo + sitegen orchestration.
//!
//! Single-flight build queue. Coalesces requests that pile up while a build
//! is running so a flurry of saves collapses to one rebuild. On success
//! pushes `DevMsg::Reload`, on failure pushes `DevMsg::Error` with the
//! captured cargo / sitegen output so the browser can render an overlay.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::process::Command;
use tokio::sync::{broadcast, mpsc};

use super::server::DevMsg;

#[derive(Debug, Clone)]
pub enum BuildRequest {
    /// Drafts / asset path: re-run sitegen, no cargo build.
    SitegenOnly { only: Vec<String> },
    /// Source change: cargo build + sitegen.
    CargoAndSitegen { only: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct BuildArgs {
    pub dist: PathBuf,
    pub drafts: PathBuf,
    pub app_crate: String,
    pub sitegen_bin: String,
}

pub async fn run_loop(
    mut rx: mpsc::UnboundedReceiver<BuildRequest>,
    args: Arc<BuildArgs>,
    events: broadcast::Sender<DevMsg>,
) {
    while let Some(first) = rx.recv().await {
        // Coalesce any others queued while we were idle.
        let mut req = first;
        while let Ok(more) = rx.try_recv() {
            req = coalesce(req, more);
        }
        let _ = events.send(DevMsg::Ok);
        match run_one(&args, &req).await {
            Ok(()) => {
                let _ = events.send(DevMsg::Reload);
            }
            Err(body) => {
                tracing::error!("build failed:\n{body}");
                let _ = events.send(DevMsg::Error { body });
            }
        }
    }
}

/// Initial / startup build — always cargo + full sitegen.
pub async fn run_full(args: &BuildArgs) -> Result<(), String> {
    cargo_build(args).await?;
    sitegen(args, &["all".to_string()]).await
}

fn coalesce(a: BuildRequest, b: BuildRequest) -> BuildRequest {
    let merge = |x: Vec<String>, y: Vec<String>| -> Vec<String> {
        let s: HashSet<String> = x.into_iter().chain(y).collect();
        if s.contains("all") {
            vec!["all".into()]
        } else {
            s.into_iter().collect()
        }
    };
    match (a, b) {
        (BuildRequest::CargoAndSitegen { only: x }, BuildRequest::CargoAndSitegen { only: y })
        | (BuildRequest::CargoAndSitegen { only: x }, BuildRequest::SitegenOnly { only: y })
        | (BuildRequest::SitegenOnly { only: x }, BuildRequest::CargoAndSitegen { only: y }) => {
            BuildRequest::CargoAndSitegen { only: merge(x, y) }
        }
        (BuildRequest::SitegenOnly { only: x }, BuildRequest::SitegenOnly { only: y }) => {
            BuildRequest::SitegenOnly { only: merge(x, y) }
        }
    }
}

async fn run_one(args: &BuildArgs, req: &BuildRequest) -> Result<(), String> {
    let needs_cargo = matches!(req, BuildRequest::CargoAndSitegen { .. });
    if needs_cargo {
        cargo_build(args).await?;
    }
    let only = match req {
        BuildRequest::SitegenOnly { only } | BuildRequest::CargoAndSitegen { only } => only.clone(),
    };
    sitegen(args, &only).await
}

async fn cargo_build(args: &BuildArgs) -> Result<(), String> {
    let started = std::time::Instant::now();
    let output = Command::new("cargo")
        .args([
            "build",
            "--profile",
            "dev-fast",
            "-p",
            &args.app_crate,
            "--bin",
            &args.sitegen_bin,
            "--features",
            "sitegen",
        ])
        .output()
        .await
        .map_err(|e| format!("spawn cargo: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("$ cargo build (dev-fast)\n\n{stdout}\n{stderr}"));
    }
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        "cargo build ok"
    );
    Ok(())
}

async fn sitegen(args: &BuildArgs, only: &[String]) -> Result<(), String> {
    let bin_paths = [
        PathBuf::from("target/dev-fast").join(&args.sitegen_bin),
        PathBuf::from("target/debug").join(&args.sitegen_bin),
    ];
    let bin = bin_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("sitegen binary missing: tried {bin_paths:?}"))?;

    let started = std::time::Instant::now();
    let output = Command::new(bin)
        .arg("--only")
        .arg(only.join(","))
        .arg("--drafts")
        .arg(&args.drafts)
        .arg(&args.dist)
        .output()
        .await
        .map_err(|e| format!("spawn sitegen: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "$ {} --only {} --drafts {} {}\n\n{stdout}\n{stderr}",
            bin.display(),
            only.join(","),
            args.drafts.display(),
            args.dist.display(),
        ));
    }
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        only = ?only,
        "sitegen ok"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_collapses_to_cargo_when_either_side_needs_it() {
        let a = BuildRequest::SitegenOnly {
            only: vec!["home".into()],
        };
        let b = BuildRequest::CargoAndSitegen {
            only: vec!["posts".into()],
        };
        let merged = coalesce(a, b);
        assert!(matches!(merged, BuildRequest::CargoAndSitegen { .. }));
    }

    #[test]
    fn coalesce_collapses_only_set_to_all() {
        let a = BuildRequest::SitegenOnly {
            only: vec!["home".into()],
        };
        let b = BuildRequest::SitegenOnly {
            only: vec!["all".into(), "posts".into()],
        };
        let merged = coalesce(a, b);
        match merged {
            BuildRequest::SitegenOnly { only } => assert_eq!(only, vec!["all"]),
            _ => panic!("expected SitegenOnly"),
        }
    }
}
