//! Local-draft loader for the dev loop.
//!
//! Markdown files under `drafts/<slug>.md` (configurable) become posts that
//! merge into the Hashnode-fetched feed. The dev daemon watches this dir and
//! re-runs sitegen on save; the live blog never sees these.
//!
//! Format: a YAML frontmatter block, then markdown body.
//!
//! ```text
//! ---
//! title: My draft
//! slug: my-draft
//! brief: Short blurb shown on the index card.
//! publishedAt: "2026-04-25T12:00:00Z"      # optional, defaults to now
//! readTimeInMinutes: 4                      # optional
//! coverImageUrl: "https://…/cover.png"      # optional
//! tags:
//!   - { name: Rust, slug: rust }
//!   - { name: Dev,  slug: dev }
//! author:                                   # optional, defaults to drzln
//!   name: drzln
//!   username: drzln
//! seo:                                      # optional
//!   title: Custom <title> override
//!   description: Custom <meta description>
//! ---
//!
//! Markdown body here.
//! ```

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use pulldown_cmark::{Options, Parser};
use serde::Deserialize;

use crate::entities::{Author, Post, PostSummary, Seo, Tag};

/// One on-disk draft, materialized as both shapes the renderer expects.
#[derive(Debug, Clone)]
pub struct LoadedDraft {
    pub summary: PostSummary,
    pub post: Post,
}

/// Load every `*.md` file under `dir` as a draft. Returns posts ordered by
/// `published_at` descending so they slot into the index next to Hashnode
/// posts. Missing dir is not an error.
pub fn load_drafts(dir: &Path) -> Result<Vec<LoadedDraft>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut drafts = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        match load_one(&path) {
            Ok(d) => drafts.push(d),
            Err(e) => tracing::warn!(path = %path.display(), error = %e, "skipping invalid draft"),
        }
    }
    drafts.sort_by(|a, b| b.summary.published_at.cmp(&a.summary.published_at));
    Ok(drafts)
}

fn load_one(path: &Path) -> Result<LoadedDraft> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let (front, body) = split_frontmatter(&raw)
        .with_context(|| format!("parse frontmatter for {}", path.display()))?;
    let fm: Frontmatter = serde_yaml_ng::from_str(front)
        .with_context(|| format!("decode YAML frontmatter for {}", path.display()))?;

    let html = render_markdown(body);
    let now = chrono::Utc::now().to_rfc3339();

    let author = fm.author.unwrap_or_else(|| Author {
        name: "drzln".to_string(),
        username: "drzln".to_string(),
        profile_picture: None,
    });

    let summary = PostSummary {
        id: format!("draft:{}", fm.slug),
        title: fm.title.clone(),
        slug: fm.slug.clone(),
        brief: fm.brief.clone().unwrap_or_default(),
        published_at: fm.published_at.clone().unwrap_or(now.clone()),
        read_time_minutes: fm.read_time_in_minutes.unwrap_or(0),
        cover_image_url: fm.cover_image_url.clone(),
        tags: fm.tags.clone().unwrap_or_default(),
        author: author.clone(),
    };

    let post = Post {
        id: summary.id.clone(),
        title: fm.title,
        slug: fm.slug,
        subtitle: fm.subtitle,
        brief: fm.brief.unwrap_or_default(),
        published_at: fm.published_at.unwrap_or(now),
        read_time_minutes: fm.read_time_in_minutes.unwrap_or(0),
        cover_image_url: fm.cover_image_url,
        content_html: html,
        content_markdown: body.to_string(),
        tags: fm.tags.unwrap_or_default(),
        author,
        seo: fm.seo,
    };

    Ok(LoadedDraft { summary, post })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frontmatter {
    title: String,
    slug: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default)]
    brief: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    read_time_in_minutes: Option<u32>,
    #[serde(default)]
    cover_image_url: Option<String>,
    #[serde(default)]
    tags: Option<Vec<Tag>>,
    #[serde(default)]
    author: Option<Author>,
    #[serde(default)]
    seo: Option<Seo>,
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let trimmed = raw.trim_start_matches('\u{feff}').trim_start();
    let after_open = trimmed
        .strip_prefix("---")
        .ok_or_else(|| anyhow!("missing leading `---` frontmatter delimiter"))?
        .trim_start_matches('\n');
    let close = after_open
        .find("\n---")
        .ok_or_else(|| anyhow!("missing closing `---` frontmatter delimiter"))?;
    let front = &after_open[..close];
    let body = after_open[close + 4..].trim_start_matches(['\n', '\r']);
    Ok((front, body))
}

fn render_markdown(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(md, opts);
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_basic_frontmatter() {
        let raw = "---\ntitle: Hi\nslug: hi\n---\n\nBody here.\n";
        let (front, body) = split_frontmatter(raw).unwrap();
        assert!(front.contains("title: Hi"));
        assert_eq!(body, "Body here.\n");
    }

    #[test]
    fn rejects_missing_delimiters() {
        assert!(split_frontmatter("no frontmatter").is_err());
        assert!(split_frontmatter("---\ntitle: x\nbody").is_err());
    }

    #[test]
    fn renders_markdown_to_html() {
        let html = render_markdown("# Hello\n\nWorld.");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<p>World.</p>"));
    }
}
