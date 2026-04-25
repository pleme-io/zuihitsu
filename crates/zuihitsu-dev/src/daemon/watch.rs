//! File watcher.
//!
//! Wraps notify + notify-debouncer-mini, classifies each path against the
//! known dirs, emits a typed `Change` so the router doesn't string-match
//! itself.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub enum Change {
    /// CSS file under `style/` — copy + push CSS swap message.
    Css(PathBuf),
    /// Asset under `public/` — copy + push reload.
    PublicAsset(PathBuf),
    /// Markdown file under `drafts/` — sitegen --only home,posts,tags,feeds.
    Drafts,
    /// Rust source under `crates/zuihitsu-app/src/` — full cargo + sitegen.
    Src,
}

/// Classify an absolute path against the watched roots. Returns None for
/// paths we don't recognise (build artefacts, editor temp files in dirs we
/// don't watch, etc).
pub fn classify(path: &Path, root: &Path) -> Option<Change> {
    let rel = path.strip_prefix(root).ok()?;
    let rel_str = rel.to_string_lossy().replace('\\', "/");

    if rel_str.starts_with("style/") {
        return is_ext(rel, "css").then(|| Change::Css(path.to_path_buf()));
    }
    if rel_str.starts_with("public/") {
        return Some(Change::PublicAsset(path.to_path_buf()));
    }
    if rel_str.starts_with("drafts/") && is_ext(rel, "md") {
        return Some(Change::Drafts);
    }
    if rel_str.starts_with("crates/zuihitsu-app/src/") && is_ext(rel, "rs") {
        return Some(Change::Src);
    }
    None
}

fn is_ext(path: &Path, want: &str) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some(want)
}

pub fn spawn_watcher(
    root: PathBuf,
    tx: UnboundedSender<Change>,
) -> Result<Debouncer<RecommendedWatcher>> {
    let cb_root = root.clone();
    let cb_tx = tx.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(120),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                for ev in events {
                    if let Some(change) = classify(&ev.path, &cb_root) {
                        let _ = cb_tx.send(change);
                    }
                }
            }
            Err(e) => tracing::error!(error = %e, "watch error"),
        },
    )
    .context("init debouncer")?;

    for sub in ["style", "public", "drafts", "crates/zuihitsu-app/src"] {
        let p = root.join(sub);
        if !p.exists() {
            std::fs::create_dir_all(&p).ok();
        }
        debouncer
            .watcher()
            .watch(&p, RecursiveMode::Recursive)
            .with_context(|| format!("watch {}", p.display()))?;
    }

    Ok(debouncer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_css() {
        let root = PathBuf::from("/repo");
        let change = classify(&root.join("style/main.css"), &root).unwrap();
        assert!(matches!(change, Change::Css(_)));
    }

    #[test]
    fn ignores_non_css_in_style_dir() {
        let root = PathBuf::from("/repo");
        assert!(classify(&root.join("style/notes.txt"), &root).is_none());
    }

    #[test]
    fn classifies_drafts() {
        let root = PathBuf::from("/repo");
        let change = classify(&root.join("drafts/foo.md"), &root).unwrap();
        assert!(matches!(change, Change::Drafts));
    }

    #[test]
    fn classifies_src() {
        let root = PathBuf::from("/repo");
        let change = classify(
            &root.join("crates/zuihitsu-app/src/static_render/mod.rs"),
            &root,
        )
        .unwrap();
        assert!(matches!(change, Change::Src));
    }

    #[test]
    fn classifies_public() {
        let root = PathBuf::from("/repo");
        let change = classify(&root.join("public/manifest.json"), &root).unwrap();
        assert!(matches!(change, Change::PublicAsset(_)));
    }

    #[test]
    fn ignores_unrelated_paths() {
        let root = PathBuf::from("/repo");
        assert!(classify(&root.join("target/foo"), &root).is_none());
        assert!(classify(&root.join("README.md"), &root).is_none());
    }
}
