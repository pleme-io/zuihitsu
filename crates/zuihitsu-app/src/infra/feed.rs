//! Sitemap + RSS generation.
//!
//! Exposes pure string builders (`build_sitemap`, `build_rss`) that both the
//! SSR axum routes and the sitegen binary call. Axum handlers live in this
//! file under `#[cfg(feature = "ssr")]`; sitegen uses the pure builders to
//! write files to disk.

use crate::entities::PostSummary;
use crate::infra::graphql::client::Hashnode;
use crate::infra::utils::xml::xml_escape;

const DEFAULT_SITE_URL: &str = "https://blog.quero.cloud";

pub fn site_url() -> String {
    std::env::var("ZUIHITSU_SITE_URL").unwrap_or_else(|_| DEFAULT_SITE_URL.to_string())
}

pub async fn fetch_all_posts(client: &Hashnode) -> anyhow::Result<Vec<PostSummary>> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = client.list_posts(cursor.as_deref(), 50).await?;
        all.extend(page.posts);
        if !page.has_next {
            break;
        }
        cursor = page.next_cursor;
    }
    Ok(all)
}

pub fn build_sitemap(posts: &[PostSummary], site_url: &str) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    xml.push_str(&format!(
        "  <url><loc>{site_url}/</loc><changefreq>daily</changefreq></url>\n"
    ));
    xml.push_str(&format!(
        "  <url><loc>{site_url}/about</loc><changefreq>monthly</changefreq></url>\n"
    ));
    for p in posts {
        xml.push_str(&format!(
            "  <url><loc>{site_url}/posts/{}</loc>",
            xml_escape(&p.slug)
        ));
        if !p.published_at.is_empty() {
            xml.push_str(&format!(
                "<lastmod>{}</lastmod>",
                xml_escape(&p.published_at)
            ));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    xml
}

pub fn build_rss(posts: &[PostSummary], site_url: &str) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n",
    );
    xml.push_str("<title>zuihitsu · 随筆</title>\n");
    xml.push_str(&format!("<link>{site_url}/</link>\n"));
    xml.push_str(
        "<description>Personal tech essays — Rust, infrastructure, systems</description>\n",
    );
    for p in posts {
        xml.push_str("<item>\n");
        xml.push_str(&format!("  <title>{}</title>\n", xml_escape(&p.title)));
        xml.push_str(&format!(
            "  <link>{site_url}/posts/{}</link>\n",
            xml_escape(&p.slug)
        ));
        xml.push_str(&format!(
            "  <guid isPermaLink=\"true\">{site_url}/posts/{}</guid>\n",
            xml_escape(&p.slug)
        ));
        if !p.published_at.is_empty() {
            xml.push_str(&format!(
                "  <pubDate>{}</pubDate>\n",
                xml_escape(&p.published_at)
            ));
        }
        xml.push_str(&format!(
            "  <description>{}</description>\n",
            xml_escape(&p.brief)
        ));
        xml.push_str("</item>\n");
    }
    xml.push_str("</channel></rss>\n");
    xml
}

// ---------- axum handlers (SSR path) ----------

#[cfg(feature = "ssr")]
pub async fn sitemap_xml() -> axum::response::Response {
    use axum::http::{StatusCode, header};
    use axum::response::IntoResponse;

    let client = match Hashnode::from_env() {
        Ok(c) => c,
        Err(e) => return error_response(format!("hashnode init: {e}")),
    };
    let posts = match fetch_all_posts(&client).await {
        Ok(p) => p,
        Err(e) => return error_response(format!("hashnode list: {e}")),
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        build_sitemap(&posts, &site_url()),
    )
        .into_response()
}

#[cfg(feature = "ssr")]
pub async fn rss_xml() -> axum::response::Response {
    use axum::http::{StatusCode, header};
    use axum::response::IntoResponse;

    let client = match Hashnode::from_env() {
        Ok(c) => c,
        Err(e) => return error_response(format!("hashnode init: {e}")),
    };
    let page = match client.list_posts(None, 20).await {
        Ok(p) => p,
        Err(e) => return error_response(format!("hashnode list: {e}")),
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        build_rss(&page.posts, &site_url()),
    )
        .into_response()
}

#[cfg(feature = "ssr")]
fn error_response(msg: String) -> axum::response::Response {
    use axum::http::{StatusCode, header};
    use axum::response::IntoResponse;
    tracing::error!(error = %msg, "feed endpoint failed");
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CONTENT_TYPE, "text/plain")],
        "feed temporarily unavailable",
    )
        .into_response()
}
