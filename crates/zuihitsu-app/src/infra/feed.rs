//! Sitemap + RSS generation. SSR-only — these routes are wired in `main.rs`.

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

use crate::infra::graphql::client::Hashnode;

const SITE_URL: &str = "https://blog.pleme.io";

pub async fn sitemap_xml() -> impl IntoResponse {
    let client = match Hashnode::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "hashnode client init failed");
            return error_response();
        }
    };
    let mut cursor: Option<String> = None;
    let mut entries = Vec::new();
    loop {
        match client.list_posts(cursor.as_deref(), 50).await {
            Ok(page) => {
                for p in page.posts {
                    entries.push((p.slug, p.published_at));
                }
                if page.has_next {
                    cursor = page.next_cursor;
                } else {
                    break;
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "hashnode list_posts failed for sitemap");
                break;
            }
        }
    }

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );
    xml.push_str(&format!(
        "  <url><loc>{SITE_URL}/</loc><changefreq>daily</changefreq></url>\n"
    ));
    xml.push_str(&format!(
        "  <url><loc>{SITE_URL}/about</loc><changefreq>monthly</changefreq></url>\n"
    ));
    for (slug, published_at) in entries {
        xml.push_str(&format!(
            "  <url><loc>{SITE_URL}/posts/{slug}</loc>"
        ));
        if !published_at.is_empty() {
            xml.push_str(&format!("<lastmod>{published_at}</lastmod>"));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

pub async fn rss_xml() -> impl IntoResponse {
    let client = match Hashnode::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "hashnode client init failed");
            return error_response();
        }
    };
    let page = match client.list_posts(None, 20).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "hashnode list_posts failed for rss");
            return error_response();
        }
    };

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel>
"#,
    );
    xml.push_str("<title>zuihitsu · 随筆</title>\n");
    xml.push_str(&format!("<link>{SITE_URL}/</link>\n"));
    xml.push_str("<description>Personal tech essays</description>\n");
    for p in page.posts {
        xml.push_str("<item>\n");
        xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&p.title)));
        xml.push_str(&format!(
            "  <link>{SITE_URL}/posts/{}</link>\n",
            xml_escape(&p.slug)
        ));
        xml.push_str(&format!(
            "  <guid isPermaLink=\"true\">{SITE_URL}/posts/{}</guid>\n",
            xml_escape(&p.slug)
        ));
        if !p.published_at.is_empty() {
            xml.push_str(&format!("  <pubDate>{}</pubDate>\n", xml_escape(&p.published_at)));
        }
        xml.push_str(&format!("  <description>{}</description>\n", xml_escape(&p.brief)));
        xml.push_str("</item>\n");
    }
    xml.push_str("</channel></rss>\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

fn error_response() -> axum::response::Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CONTENT_TYPE, "text/plain")],
        "feed temporarily unavailable",
    )
        .into_response()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
