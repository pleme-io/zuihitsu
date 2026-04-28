//! zuihitsu static site generator.
//!
//! Fetches every post / tag from Hashnode, renders the full site to static
//! HTML, writes `dist/` in a layout Cloudflare Pages expects. Copies
//! `public/` plus the source CSS files alongside so wrangler can upload the
//! directory as-is.
//!
//! Usage:
//!
//! ```text
//! zuihitsu-sitegen [--only <set>] [--drafts <dir>] [DIST_DIR]
//! ```
//!
//! `--only` accepts a comma-separated subset of
//! `home,about,not_found,posts,tags,feeds,assets,all` (default `all`). The
//! dev daemon uses this to avoid full re-renders on small edits.
//!
//! `--drafts <dir>` merges `*.md` files from a local drafts directory into
//! the post list — they appear on the index, on tag pages, and at
//! `/posts/<slug>/`. Hashnode never sees them. See
//! `infra/draft.rs` for the frontmatter format.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use futures::future::try_join_all;
use zuihitsu_app::entities::{Post, PostSummary};
use zuihitsu_app::infra::draft::{LoadedDraft, load_drafts};
use zuihitsu_app::infra::feed::{build_rss, build_sitemap, fetch_all_posts, site_url};
use zuihitsu_app::infra::graphql::client::Hashnode;
use zuihitsu_app::static_render::{
    render_about, render_home, render_not_found, render_post, render_tag,
};

#[derive(Parser, Debug)]
#[command(
    name = "zuihitsu-sitegen",
    about = "render zuihitsu's static site into a directory"
)]
struct Args {
    /// Output directory.
    #[arg(default_value = "dist")]
    out: PathBuf,

    /// Comma-separated render set: home,about,not_found,posts,tags,feeds,assets,all.
    #[arg(long, default_value = "all")]
    only: String,

    /// Merge markdown drafts from this directory into the post list.
    #[arg(long)]
    drafts: Option<PathBuf>,
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
enum Target {
    Home,
    About,
    NotFound,
    Posts,
    Tags,
    Feeds,
    Assets,
}

impl Target {
    const ALL: &'static [Target] = &[
        Target::Home,
        Target::About,
        Target::NotFound,
        Target::Posts,
        Target::Tags,
        Target::Feeds,
        Target::Assets,
    ];

    fn parse(s: &str) -> Result<Self> {
        Ok(match s.trim() {
            "home" => Target::Home,
            "about" => Target::About,
            "not_found" | "not-found" | "404" => Target::NotFound,
            "posts" => Target::Posts,
            "tags" => Target::Tags,
            "feeds" => Target::Feeds,
            "assets" => Target::Assets,
            other => anyhow::bail!("unknown --only target: {other}"),
        })
    }
}

fn parse_only(s: &str) -> Result<HashSet<Target>> {
    if s.trim() == "all" {
        return Ok(Target::ALL.iter().copied().collect());
    }
    s.split(',').map(Target::parse).collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zuihitsu_sitegen=debug".into()),
        )
        .init();

    let args = Args::parse();
    let only = parse_only(&args.only)?;
    let out = &args.out;
    std::fs::create_dir_all(out).context("create out dir")?;

    let site = site_url();
    tracing::info!(%site, out = %out.display(), targets = ?args.only, drafts = ?args.drafts, "sitegen starting");

    // Only home / posts / tags / feeds need Hashnode summaries. about,
    // not_found, and assets are static — render them without any network
    // (or cache) round-trip. Lets `--only assets` succeed offline with no
    // Hashnode account present.
    let needs_hashnode = [Target::Home, Target::Posts, Target::Tags, Target::Feeds]
        .iter()
        .any(|t| only.contains(t));

    let mut summaries: Vec<PostSummary> = Vec::new();
    let mut draft_posts: std::collections::HashMap<String, Post> = std::collections::HashMap::new();
    let client = if needs_hashnode {
        Some(Hashnode::from_env().context("hashnode client")?)
    } else {
        None
    };

    if needs_hashnode {
        let c = client
            .as_ref()
            .expect("client constructed when needs_hashnode");
        summaries = fetch_all_posts(c).await.context("fetch posts")?;
        tracing::info!(count = summaries.len(), "fetched hashnode posts");

        // Drafts merge in here. We keep the rendered detail pages for each
        // draft in a side-map so render_post() doesn't have to re-fetch them
        // via a GraphQL call that would never succeed.
        let drafts = match &args.drafts {
            Some(dir) => load_drafts(dir).context("load drafts")?,
            None => Vec::new(),
        };
        if !drafts.is_empty() {
            tracing::info!(count = drafts.len(), "merged local drafts");
            for d in &drafts {
                summaries.push(d.summary.clone());
            }
            summaries.sort_by(|a, b| b.published_at.cmp(&a.published_at));
        }
        draft_posts = drafts
            .into_iter()
            .map(|LoadedDraft { post, .. }| (post.slug.clone(), post))
            .collect();
    }

    // ── Static pages ───────────────────────────────────────────────────────
    if only.contains(&Target::Home) {
        write(&out.join("index.html"), &render_home(&summaries))?;
    }
    if only.contains(&Target::About) {
        write(&out.join("about/index.html"), &render_about())?;
    }
    if only.contains(&Target::NotFound) {
        write(&out.join("404.html"), &render_not_found())?;
    }

    // ── Per-post pages (parallel fetches) ──────────────────────────────────
    if only.contains(&Target::Posts) {
        let c = client
            .as_ref()
            .expect("Posts target requires Hashnode client");
        let fetches = summaries.iter().map(|summary| {
            let client = c.clone();
            let slug = summary.slug.clone();
            let drafts = &draft_posts;
            async move {
                if let Some(p) = drafts.get(&slug) {
                    return Ok::<(String, Option<Post>), anyhow::Error>((slug, Some(p.clone())));
                }
                let p = client
                    .get_post(&slug)
                    .await
                    .with_context(|| format!("fetch post {slug}"))?;
                Ok((slug, p))
            }
        });
        let results = try_join_all(fetches).await?;

        let mut wrote = 0usize;
        let mut skipped = 0usize;
        for (slug, maybe_post) in results {
            match maybe_post {
                Some(post) => {
                    let path = out.join("posts").join(&slug).join("index.html");
                    write(&path, &render_post(&post, &site))?;
                    wrote += 1;
                }
                None => {
                    tracing::warn!(%slug, "post disappeared between list and get");
                    skipped += 1;
                }
            }
        }
        tracing::info!(wrote, skipped, "rendered posts");
    }

    // ── Per-tag pages ──────────────────────────────────────────────────────
    if only.contains(&Target::Tags) {
        let c = client
            .as_ref()
            .expect("Tags target requires Hashnode client");
        let mut tags = c.list_tags().await.context("list tags")?;
        // Pull tag definitions from drafts that referenced novel tags too.
        for s in &summaries {
            for t in &s.tags {
                if !tags.iter().any(|existing| existing.slug == t.slug) {
                    tags.push(t.clone());
                }
            }
        }
        tags.sort_by(|a, b| a.slug.cmp(&b.slug));
        tags.dedup_by(|a, b| a.slug == b.slug);

        for tag in &tags {
            let grouped: Vec<PostSummary> = summaries
                .iter()
                .filter(|p| p.tags.iter().any(|t| t.slug == tag.slug))
                .cloned()
                .collect();
            let path = out.join("tags").join(&tag.slug).join("index.html");
            write(&path, &render_tag(tag, &grouped))?;
        }
        tracing::info!(count = tags.len(), "rendered tag pages");
    }

    // ── Feeds ──────────────────────────────────────────────────────────────
    if only.contains(&Target::Feeds) {
        write(&out.join("sitemap.xml"), &build_sitemap(&summaries, &site))?;
        write(&out.join("rss.xml"), &build_rss(&summaries, &site))?;
    }

    // ── Static assets ──────────────────────────────────────────────────────
    if only.contains(&Target::Assets) {
        copy_dir("public", out).context("copy public/")?;
        copy_file("style/main.css", &out.join("main.css"))?;
        copy_file("style/ishou.css", &out.join("ishou.css"))?;
    }

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
