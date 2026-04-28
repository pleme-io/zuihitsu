//! `zuihitsu-dev daemon` — the watch / build / serve loop.
//!
//! Architecture:
//!
//! ```text
//!   notify (style/, public/, drafts/, crates/zuihitsu-app/src/)
//!         │
//!         ▼  Change
//!   change_router ────► broadcast::Sender<DevMsg>
//!         │              ▲          (CSS / asset → directly)
//!         ▼              │
//!   build_queue ─────────┘ (src / drafts → cargo+sitegen → reload | error)
//!         │
//!         ▼
//!   axum :3000          (serves dist/ with auto-injected livereload script)
//!   axum :3000/__dev/ws (broadcast events to all browsers)
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

mod build;
mod poller;
mod server;
mod watch;

use build::{BuildArgs, BuildRequest};
use server::{DevMsg, ServerState};
use watch::Change;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Repo root (the directory holding flake.nix, Cargo.toml, etc).
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    /// Output directory served by the dev server.
    #[arg(long, default_value = "dist")]
    pub dist: PathBuf,
    /// Drafts directory (markdown files merged with Hashnode posts).
    #[arg(long, default_value = "drafts")]
    pub drafts: PathBuf,
    /// Hashnode response cache directory.
    #[arg(long, default_value = ".cache/hashnode")]
    pub cache: PathBuf,
    /// HTTP server port.
    #[arg(long, default_value_t = 3000)]
    pub port: u16,
    /// Disable the Hashnode background poller.
    #[arg(long)]
    pub no_poll: bool,
    /// Hashnode poll interval (seconds).
    #[arg(long, default_value_t = 30)]
    pub poll_interval: u64,
    /// Cargo crate name to build (the workspace member with the sitegen bin).
    #[arg(long, default_value = "zuihitsu-app")]
    pub app_crate: String,
    /// Sitegen binary name.
    #[arg(long, default_value = "zuihitsu-sitegen")]
    pub sitegen_bin: String,
}

pub async fn run(args: Args) -> Result<()> {
    let root = args
        .root
        .canonicalize()
        .unwrap_or_else(|_| args.root.clone());
    std::env::set_current_dir(&root)?;

    // Make sure the dirs we watch / write to actually exist.
    let cache_abs = root.join(&args.cache);
    std::fs::create_dir_all(&cache_abs)?;
    std::fs::create_dir_all(&args.dist)?;
    std::fs::create_dir_all(&args.drafts)?;

    // SAFETY: still single-threaded — no tasks spawned yet, so no concurrent
    // env reads. Edition 2024 marks set_var unsafe to flag the multi-thread
    // hazard. Both vars propagate to every cargo / sitegen subprocess.
    unsafe {
        std::env::set_var("ZUIHITSU_DEV_LINKED_CSS", "1");
        std::env::set_var("ZUIHITSU_HASHNODE_CACHE_DIR", &cache_abs);
    }

    let bargs = Arc::new(BuildArgs {
        dist: args.dist.clone(),
        drafts: args.drafts.clone(),
        app_crate: args.app_crate.clone(),
        sitegen_bin: args.sitegen_bin.clone(),
    });

    // Initial build — block so the first browser connect has something real
    // to look at. Keep going on failure; the overlay will surface it.
    tracing::info!("warming cache + initial sitegen…");
    if let Err(body) = build::run_full(bargs.as_ref()).await {
        tracing::error!("initial sitegen failed:\n{body}");
    }

    let (events_tx, _) = broadcast::channel::<DevMsg>(64);
    let (changes_tx, mut changes_rx) = mpsc::unbounded_channel::<Change>();
    let (build_tx, build_rx) = mpsc::unbounded_channel::<BuildRequest>();

    let _watcher = watch::spawn_watcher(root.clone(), changes_tx)?;

    {
        let events = events_tx.clone();
        let bargs = bargs.clone();
        tokio::spawn(async move { build::run_loop(build_rx, bargs, events).await });
    }

    {
        let events = events_tx.clone();
        let dist = args.dist.clone();
        let build_tx = build_tx.clone();
        tokio::spawn(async move {
            while let Some(change) = changes_rx.recv().await {
                handle_change(change, &dist, &events, &build_tx).await;
            }
        });
    }

    if !args.no_poll {
        let interval = std::time::Duration::from_secs(args.poll_interval);
        let cache = cache_abs.clone();
        let build_tx = build_tx.clone();
        tokio::spawn(async move { poller::run(interval, cache, build_tx).await });
    }

    let state = ServerState {
        dist: args.dist.clone(),
        events: events_tx,
    };
    println!(
        "\n  zuihitsu-dev  →  http://127.0.0.1:{}\n  watch        →  style/, public/, drafts/, crates/zuihitsu-app/src/\n  cache        →  {}\n",
        args.port,
        cache_abs.display()
    );
    server::serve(state, args.port).await
}

async fn handle_change(
    change: Change,
    dist: &Path,
    events: &broadcast::Sender<DevMsg>,
    build_tx: &mpsc::UnboundedSender<BuildRequest>,
) {
    match change {
        Change::Css(path) => {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                return;
            };
            let dest = dist.join(name);
            if let Err(e) = tokio::fs::copy(&path, &dest).await {
                tracing::warn!(error = %e, src = %path.display(), "css copy failed");
                return;
            }
            tracing::info!(file = %name, "css → swap");
            let _ = events.send(DevMsg::Css {
                path: format!("/{name}"),
            });
        }
        Change::PublicAsset(path) => {
            // Strip the `public/` prefix relative to the repo root so
            // `public/sub/foo.png` lands at `dist/sub/foo.png`.
            let rel = path
                .components()
                .skip_while(|c| c.as_os_str() != "public")
                .skip(1)
                .collect::<PathBuf>();
            let rel = if rel.as_os_str().is_empty() {
                path.file_name().map(PathBuf::from).unwrap_or_default()
            } else {
                rel
            };
            let dest = dist.join(&rel);
            if let Some(parent) = dest.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if let Err(e) = tokio::fs::copy(&path, &dest).await {
                tracing::warn!(error = %e, src = %path.display(), "public asset copy failed");
                return;
            }
            tracing::info!(file = %rel.display(), "asset → reload");
            let _ = events.send(DevMsg::Reload);
        }
        Change::Drafts => {
            tracing::info!("drafts changed → sitegen (home,posts,tags,feeds)");
            let _ = build_tx.send(BuildRequest::SitegenOnly {
                only: ["home", "posts", "tags", "feeds"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            });
        }
        Change::Src => {
            tracing::info!("src changed → cargo build + full sitegen");
            let _ = build_tx.send(BuildRequest::CargoAndSitegen {
                only: vec!["all".into()],
            });
        }
    }
}
