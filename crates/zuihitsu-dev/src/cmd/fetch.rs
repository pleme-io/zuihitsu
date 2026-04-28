//! `zuihitsu-dev fetch` — invalidate the Hashnode disk cache + re-warm.
//!
//! Useful when you've published a post, edited tags, or just want a clean
//! pull. The warm pass is a `cargo run` of the sitegen binary with
//! `ZUIHITSU_HASHNODE_CACHE_DIR` pointed at the new (empty) directory.

use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Cache directory to invalidate.
    #[arg(long, default_value = ".cache/hashnode")]
    pub cache: PathBuf,
    /// Skip invalidation; only run the warm pass if cache is missing.
    #[arg(long)]
    pub no_invalidate: bool,
    /// Drafts directory to include during the warm pass.
    #[arg(long, default_value = "drafts")]
    pub drafts: PathBuf,
    /// Output dir for the warm pass.
    #[arg(long, default_value = "dist")]
    pub dist: PathBuf,
    /// Cargo crate name (zuihitsu-app by default).
    #[arg(long, default_value = "zuihitsu-app")]
    pub app_crate: String,
    /// Sitegen binary name.
    #[arg(long, default_value = "zuihitsu-sitegen")]
    pub sitegen_bin: String,
}

pub async fn run(args: Args) -> Result<()> {
    if !args.no_invalidate && args.cache.exists() {
        std::fs::remove_dir_all(&args.cache)
            .with_context(|| format!("remove {}", args.cache.display()))?;
        println!("[fetch] invalidated {}", args.cache.display());
    }
    std::fs::create_dir_all(&args.cache)
        .with_context(|| format!("mkdir {}", args.cache.display()))?;

    // SAFETY: single-threaded program point — set_var is unsafe in Rust 2024
    // because it can race with reads on other threads, but at this stage we
    // haven't spawned any tasks that read the env yet.
    unsafe {
        std::env::set_var("ZUIHITSU_HASHNODE_CACHE_DIR", &args.cache);
    }

    println!(
        "[fetch] running sitegen to warm cache → {}",
        args.cache.display()
    );

    let drafts = args.drafts.to_string_lossy();
    let dist = args.dist.to_string_lossy();
    let status = tokio::process::Command::new("cargo")
        .args([
            "run",
            "--profile",
            "dev-fast",
            "-p",
            &args.app_crate,
            "--bin",
            &args.sitegen_bin,
            "--features",
            "sitegen",
            "--",
            "--drafts",
            drafts.as_ref(),
            dist.as_ref(),
        ])
        .status()
        .await
        .context("spawn cargo run")?;

    if !status.success() {
        anyhow::bail!("sitegen exited non-zero");
    }
    println!("[fetch] cache warmed");
    Ok(())
}
