//! Pure-string HTML renderers for the sitegen binary.
//!
//! These are independent of Leptos — they emit complete HTML documents by
//! string interpolation. A single-author tech blog has ~5 template shapes;
//! hand-rolled templates are predictable, fast, and easy to reason about.
//!
//! All templates share `shell()`, which inlines `style/ishou.css` (design
//! tokens from the ishou flake input) + `style/main.css` (zuihitsu-specific
//! layout), wires the PWA manifest, and renders the standard header/footer.

use crate::entities::{Post, PostSummary, Tag};
use crate::infra::utils::format::{format_short_date, reading_time_label};
use crate::infra::utils::xml::xml_escape;

const ISHOU_CSS: &str = include_str!("../../../../style/ishou.css");
const APP_CSS: &str = include_str!("../../../../style/main.css");

/// Renders the `<style>` / `<link>` block that goes in `<head>`.
///
/// Production: inlines both CSS files into the document for one-round-trip
/// rendering — Cloudflare Pages serves the page and the styles in a single
/// HTTP response. Sitegen always copies the source CSS into `dist/` too.
///
/// Dev (`ZUIHITSU_DEV_LINKED_CSS=1`): emits `<link>` tags pointing at
/// `/ishou.css` and `/main.css`. The dev daemon (zuihitsu-dev) watches the
/// CSS sources, copies them into `dist/`, and pushes a `css` WebSocket event
/// to the browser, which swaps the link href with a cache-buster — no full
/// reload, no Rust rebuild, no flash.
fn css_block() -> String {
    if std::env::var_os("ZUIHITSU_DEV_LINKED_CSS").is_some() {
        r#"<link rel="stylesheet" href="/ishou.css"/>
<link rel="stylesheet" href="/main.css"/>"#
            .to_string()
    } else {
        format!("<style>{ISHOU_CSS}</style><style>{APP_CSS}</style>")
    }
}

pub struct Meta<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub canonical: Option<&'a str>,
    pub og_image: Option<&'a str>,
    pub og_type: &'a str,
}

impl<'a> Meta<'a> {
    pub fn page(title: &'a str, description: &'a str) -> Self {
        Self {
            title,
            description,
            canonical: None,
            og_image: None,
            og_type: "website",
        }
    }
}

fn shell(meta: &Meta, body: &str) -> String {
    let canonical_tag = meta
        .canonical
        .map(|u| format!(r#"<link rel="canonical" href="{}"/>"#, xml_escape(u)))
        .unwrap_or_default();
    let og_image_tag = meta
        .og_image
        .map(|u| format!(r#"<meta property="og:image" content="{}"/>"#, xml_escape(u)))
        .unwrap_or_default();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<meta name="theme-color" content="#2e3440"/>
<meta name="color-scheme" content="dark"/>
<title>{title}</title>
<meta name="description" content="{description}"/>
<meta property="og:type" content="{og_type}"/>
<meta property="og:title" content="{title}"/>
<meta property="og:description" content="{description}"/>
{og_image}
{canonical}
<link rel="manifest" href="/manifest.json"/>
<link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
<link rel="alternate" type="application/rss+xml" title="zuihitsu" href="/rss.xml"/>
{css}
</head>
<body>
<header class="z-header">
  <a class="z-brand" href="/"><span class="z-brand-kanji">随筆</span><span class="z-brand-text">zuihitsu</span></a>
  <nav class="z-nav">
    <a class="z-nav-link" href="/">Essays</a>
    <a class="z-nav-link" href="/about">About</a>
    <a class="z-nav-link" href="/rss.xml">RSS</a>
  </nav>
</header>
<main class="z-main">
{body}
</main>
<footer class="z-footer">
  <span class="z-footer-text">© 2026 · written in <a href="https://hashnode.com" rel="noopener">Hashnode</a> · served static from Cloudflare Pages</span>
  <a class="z-footer-link" href="https://github.com/pleme-io/zuihitsu" rel="noopener">source</a>
</footer>
</body>
</html>
"##,
        title = xml_escape(meta.title),
        description = xml_escape(meta.description),
        og_type = meta.og_type,
        og_image = og_image_tag,
        canonical = canonical_tag,
        css = css_block(),
        body = body,
    )
}

fn render_post_card(post: &PostSummary) -> String {
    let href = format!("/posts/{}", post.slug);
    let date = format_short_date(&post.published_at);
    let read = reading_time_label(post.read_time_minutes);
    let cover = post
        .cover_image_url
        .as_ref()
        .map(|url| {
            format!(
                r#"<a class="z-card-cover-link" href="{}"><img class="z-card-cover" src="{}" alt="" loading="lazy"/></a>"#,
                xml_escape(&href),
                xml_escape(url)
            )
        })
        .unwrap_or_default();
    let tags: String = post
        .tags
        .iter()
        .map(|t| {
            format!(
                r##"<a class="z-tag-chip" href="/tags/{slug}">#{name}</a>"##,
                slug = xml_escape(&t.slug),
                name = xml_escape(&t.name),
            )
        })
        .collect();
    format!(
        r#"<article class="z-card">
{cover}
<a class="z-card-title-link" href="{href}"><h2 class="z-card-title">{title}</h2></a>
<p class="z-card-brief">{brief}</p>
<div class="z-card-meta"><time>{date}</time><span class="z-card-dot">·</span><span>{read}</span></div>
<div class="z-card-tags">{tags}</div>
</article>"#,
        cover = cover,
        href = xml_escape(&href),
        title = xml_escape(&post.title),
        brief = xml_escape(&post.brief),
        date = xml_escape(&date),
        read = xml_escape(&read),
        tags = tags,
    )
}

pub fn render_home(posts: &[PostSummary]) -> String {
    let cards: String = posts.iter().map(render_post_card).collect();
    let body = format!(
        r#"<section class="z-hero">
<h1 class="z-hero-title">随筆</h1>
<p class="z-hero-subtitle">Essays on Rust, infrastructure, and what I'm building.</p>
</section>
<section class="z-post-list">{cards}</section>
<aside class="z-subscribe">
<h3 class="z-subscribe-title">Subscribe</h3>
<p class="z-subscribe-body">Follow new posts via <a href="/rss.xml">RSS</a>.</p>
</aside>"#
    );
    shell(
        &Meta::page(
            "zuihitsu · 随筆 — personal tech essays",
            "Rust, infrastructure, and systems essays by drzln.",
        ),
        &body,
    )
}

pub fn render_post(post: &Post, site_url: &str) -> String {
    let date = format_short_date(&post.published_at);
    let read = reading_time_label(post.read_time_minutes);
    let subtitle = post
        .subtitle
        .as_ref()
        .map(|s| format!(r#"<p class="z-article-subtitle">{}</p>"#, xml_escape(s)))
        .unwrap_or_default();
    let cover = post
        .cover_image_url
        .as_ref()
        .map(|url| {
            format!(
                r#"<img class="z-article-cover" src="{}" alt="" loading="eager"/>"#,
                xml_escape(url)
            )
        })
        .unwrap_or_default();
    let tags: String = post
        .tags
        .iter()
        .map(|t| {
            format!(
                r##"<a class="z-tag-chip" href="/tags/{slug}">#{name}</a>"##,
                slug = xml_escape(&t.slug),
                name = xml_escape(&t.name),
            )
        })
        .collect();
    let body = format!(
        r#"<article class="z-article">
<header class="z-article-header">
<h1 class="z-article-title">{title}</h1>
{subtitle}
<div class="z-article-meta"><time>{date}</time><span class="z-card-dot">·</span><span>{read}</span></div>
</header>
{cover}
<div class="z-prose">{content_html}</div>
<footer class="z-article-footer"><div class="z-card-tags">{tags}</div></footer>
</article>"#,
        title = xml_escape(&post.title),
        subtitle = subtitle,
        date = xml_escape(&date),
        read = xml_escape(&read),
        cover = cover,
        // content_html is trusted server-rendered HTML from Hashnode — do NOT escape
        content_html = post.content_html,
        tags = tags,
    );
    let title = format!("{} · zuihitsu", post.title);
    let desc = post
        .seo
        .as_ref()
        .and_then(|s| s.description.clone())
        .unwrap_or_else(|| post.brief.clone());
    let canonical = format!("{site_url}/posts/{}", post.slug);
    let meta = Meta {
        title: &title,
        description: &desc,
        canonical: Some(&canonical),
        og_image: post.cover_image_url.as_deref(),
        og_type: "article",
    };
    shell(&meta, &body)
}

pub fn render_tag(tag: &Tag, posts: &[PostSummary]) -> String {
    let cards: String = posts.iter().map(render_post_card).collect();
    let body = format!(
        r##"<h1 class="z-tag-heading">#{name}</h1>
<section class="z-post-list">{cards}</section>"##,
        name = xml_escape(&tag.name),
        cards = cards,
    );
    let title = format!("#{} · zuihitsu", tag.name);
    let desc = format!("Posts tagged #{}", tag.name);
    shell(&Meta::page(&title, &desc), &body)
}

pub fn render_about() -> String {
    let body = r#"<article class="z-prose">
<h1>About</h1>
<p>zuihitsu (<i>随筆</i>) is the classical Japanese literary genre for personal essays — fragmentary musings written as the brush follows thought. This is mine.</p>
<p>I write about Rust, distributed systems, infrastructure-as-code, and the platforms I build. Posts are authored in Hashnode and rendered by a statically-generated Rust pipeline that deploys to Cloudflare Pages.</p>
<p><a href="https://github.com/drzln" rel="noopener">GitHub</a> · <a href="/rss.xml">RSS</a></p>
</article>"#;
    shell(
        &Meta::page("About · zuihitsu", "About zuihitsu and its author."),
        body,
    )
}

pub fn render_not_found() -> String {
    let body = r#"<div class="z-empty">
<h1>404</h1>
<p>The essay you sought is not here.</p>
<p><a href="/">Return to the index</a></p>
</div>"#;
    shell(&Meta::page("Not found · zuihitsu", "Page not found."), body)
}
