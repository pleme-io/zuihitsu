//! `zuihitsu-dev draft <slug>` — scaffold a new markdown draft.
//!
//! Writes a `drafts/<slug>.md` with the YAML frontmatter the sitegen
//! drafts loader expects. The dev daemon picks it up immediately on save.

use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Slug for the draft (becomes drafts/<slug>.md).
    pub slug: String,
    /// Title (defaults to a humanized slug).
    #[arg(long)]
    pub title: Option<String>,
    /// Drafts directory.
    #[arg(long, default_value = "drafts")]
    pub dir: PathBuf,
}

pub async fn run(args: Args) -> Result<()> {
    tokio::fs::create_dir_all(&args.dir)
        .await
        .with_context(|| format!("mkdir {}", args.dir.display()))?;
    let path = args.dir.join(format!("{}.md", args.slug));
    if path.exists() {
        anyhow::bail!("{} already exists", path.display());
    }
    let title = args.title.unwrap_or_else(|| humanize(&args.slug));
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        "---\n\
title: {title}\n\
slug: {slug}\n\
brief: \n\
publishedAt: \"{now}\"\n\
readTimeInMinutes: 0\n\
tags:\n\
  - {{ name: Rust, slug: rust }}\n\
---\n\
\n\
# {title}\n\
\n\
Draft body.\n",
        title = title,
        slug = args.slug,
        now = now,
    );
    tokio::fs::write(&path, body)
        .await
        .with_context(|| format!("write {}", path.display()))?;
    println!("[draft] created {}", path.display());
    Ok(())
}

fn humanize(slug: &str) -> String {
    slug.split('-')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
