//! zuihitsu static site generator.
//!
//! Fetches every post / tag from Hashnode, renders the full site to static
//! HTML, writes `dist/` in a layout Cloudflare Pages expects. Copies
//! `public/` and `style/main.css` alongside so wrangler can upload the
//! directory as-is.
//!
//! Usage:
//!   zuihitsu-sitegen [OUT_DIR]          (default: dist)

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use zuihitsu_app::entities::PostSummary;
use zuihitsu_app::infra::feed::{build_rss, build_sitemap, fetch_all_posts, site_url};
use zuihitsu_app::infra::graphql::client::Hashnode;
use zuihitsu_app::static_render::{
    render_about, render_home, render_not_found, render_post, render_tag,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zuihitsu_sitegen=debug".into()),
        )
        .init();

    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist"));
    std::fs::create_dir_all(&out).context("create out dir")?;

    let site = site_url();
    tracing::info!(%site, out = %out.display(), "sitegen starting");

    let client = Hashnode::from_env().context("hashnode client")?;
    let posts = fetch_all_posts(&client).await.context("fetch posts")?;
    tracing::info!(count = posts.len(), "fetched posts");

    // -- static pages -------------------------------------------------------
    write(&out.join("index.html"), &render_home(&posts))?;
    write(&out.join("about/index.html"), &render_about())?;
    write(&out.join("404.html"), &render_not_found())?;

    // -- per-post pages -----------------------------------------------------
    for summary in &posts {
        match client.get_post(&summary.slug).await {
            Ok(Some(post)) => {
                let path = out.join("posts").join(&summary.slug).join("index.html");
                write(&path, &render_post(&post, &site))?;
                tracing::debug!(slug = %summary.slug, "wrote post");
            }
            Ok(None) => tracing::warn!(slug = %summary.slug, "post disappeared between list and get"),
            Err(e) => tracing::error!(slug = %summary.slug, error = %e, "fetch post failed"),
        }
    }

    // -- per-tag pages ------------------------------------------------------
    let tags = client.list_tags().await.context("list tags")?;
    for tag in &tags {
        let grouped: Vec<PostSummary> = posts
            .iter()
            .filter(|p| p.tags.iter().any(|t| t.slug == tag.slug))
            .cloned()
            .collect();
        let path = out.join("tags").join(&tag.slug).join("index.html");
        write(&path, &render_tag(tag, &grouped))?;
    }
    tracing::info!(count = tags.len(), "wrote tag pages");

    // -- feeds --------------------------------------------------------------
    write(&out.join("sitemap.xml"), &build_sitemap(&posts, &site))?;
    write(&out.join("rss.xml"), &build_rss(&posts, &site))?;

    // -- static assets ------------------------------------------------------
    copy_dir("public", &out).context("copy public/")?;
    copy_file("style/main.css", &out.join("main.css"))?;

    tracing::info!("sitegen complete");
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {parent:?}"))?;
    }
    std::fs::write(path, contents).with_context(|| format!("write {path:?}"))
}

fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let (src, dst) = (src.as_ref(), dst.as_ref());
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if src.exists() {
        std::fs::copy(src, dst).with_context(|| format!("copy {src:?} -> {dst:?}"))?;
    }
    Ok(())
}

fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.file_name().unwrap();
        let target = dst.join(rel);
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)
                .with_context(|| format!("copy {path:?} -> {target:?}"))?;
        }
    }
    Ok(())
}
