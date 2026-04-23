//! Hashnode headless GraphQL client — SSR only.
//!
//! The browser never talks to Hashnode directly. Every fetch goes through a
//! Leptos `#[server]` function in `shared::server_fns`, which constructs this
//! client fresh per-request. This keeps CORS out of the picture and keeps the
//! hydrate bundle free of reqwest.

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::entities::{Author, Post, PostPage, PostSummary, Publication, Seo, Tag};
use super::queries;

const GQL_ENDPOINT: &str = "https://gql.hashnode.com/";
const DEFAULT_HOST: &str = "drzln.hashnode.dev";

#[derive(Clone)]
pub struct Hashnode {
    host: String,
    http: Client,
}

impl Hashnode {
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("ZUIHITSU_HASHNODE_HOST")
            .unwrap_or_else(|_| DEFAULT_HOST.to_string());
        let http = Client::builder()
            .user_agent("zuihitsu/0.1 (+https://github.com/pleme-io/zuihitsu)")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("build reqwest client")?;
        Ok(Self { host, http })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    async fn query(&self, body: Value) -> Result<Value> {
        let resp = self
            .http
            .post(GQL_ENDPOINT)
            .json(&body)
            .send()
            .await
            .context("POST gql.hashnode.com")?;
        let status = resp.status();
        let json: Value = resp.json().await.context("decode hashnode response")?;
        if !status.is_success() {
            return Err(anyhow!("hashnode {status}: {json}"));
        }
        if let Some(errs) = json.get("errors")
            && !errs.is_null()
        {
            return Err(anyhow!("hashnode graphql errors: {errs}"));
        }
        Ok(json)
    }

    pub async fn list_posts(&self, cursor: Option<&str>, first: i32) -> Result<PostPage> {
        let data = self
            .query(json!({
                "query": queries::LIST_POSTS,
                "variables": { "host": self.host, "first": first, "after": cursor },
            }))
            .await?;
        parse_post_page(&data, "/data/publication/posts")
    }

    pub async fn list_posts_by_tag(
        &self,
        tag_slug: &str,
        cursor: Option<&str>,
        first: i32,
    ) -> Result<PostPage> {
        let data = self
            .query(json!({
                "query": queries::LIST_POSTS_BY_TAG,
                "variables": {
                    "host": self.host,
                    "first": first,
                    "after": cursor,
                    "tagSlug": tag_slug,
                },
            }))
            .await?;
        parse_post_page(&data, "/data/publication/posts")
    }

    pub async fn get_post(&self, slug: &str) -> Result<Option<Post>> {
        let data = self
            .query(json!({
                "query": queries::GET_POST,
                "variables": { "host": self.host, "slug": slug },
            }))
            .await?;
        let Some(node) = data.pointer("/data/publication/post") else {
            return Ok(None);
        };
        if node.is_null() {
            return Ok(None);
        }
        Ok(Some(parse_post(node)?))
    }

    pub async fn list_tags(&self) -> Result<Vec<Tag>> {
        let data = self
            .query(json!({
                "query": queries::LIST_TAGS,
                "variables": { "host": self.host },
            }))
            .await?;
        let arr = data
            .pointer("/data/publication/posts/edges")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut tags: Vec<Tag> = arr
            .iter()
            .flat_map(|edge| {
                edge.pointer("/node/tags")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            })
            .filter_map(|t| serde_json::from_value::<RawTag>(t).ok())
            .map(|t| Tag {
                name: t.name,
                slug: t.slug,
            })
            .collect();
        tags.sort_by(|a, b| a.slug.cmp(&b.slug));
        tags.dedup_by(|a, b| a.slug == b.slug);
        Ok(tags)
    }

    pub async fn get_publication(&self) -> Result<Option<Publication>> {
        let data = self
            .query(json!({
                "query": queries::GET_PUBLICATION,
                "variables": { "host": self.host },
            }))
            .await?;
        let Some(node) = data.pointer("/data/publication") else {
            return Ok(None);
        };
        if node.is_null() {
            return Ok(None);
        }
        Ok(Some(Publication {
            id: node.pointer("/id").and_then(Value::as_str).unwrap_or("").to_owned(),
            title: node
                .pointer("/title")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
            display_title: node
                .pointer("/displayTitle")
                .and_then(Value::as_str)
                .map(str::to_owned),
            about_html: node
                .pointer("/about/html")
                .and_then(Value::as_str)
                .map(str::to_owned),
            og_image_url: node
                .pointer("/ogMetaData/image")
                .and_then(Value::as_str)
                .map(str::to_owned),
            favicon_url: node
                .pointer("/favicon")
                .and_then(Value::as_str)
                .map(str::to_owned),
        }))
    }
}

fn parse_post_page(data: &Value, pointer: &str) -> Result<PostPage> {
    let conn = data
        .pointer(pointer)
        .ok_or_else(|| anyhow!("missing {pointer} in hashnode response"))?;
    let has_next = conn
        .pointer("/pageInfo/hasNextPage")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let next_cursor = conn
        .pointer("/pageInfo/endCursor")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let posts = conn
        .pointer("/edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|edge| edge.get("node").cloned())
        .map(|node| parse_post_summary(&node))
        .collect::<Result<Vec<_>>>()?;
    Ok(PostPage {
        posts,
        next_cursor,
        has_next,
    })
}

fn parse_post_summary(node: &Value) -> Result<PostSummary> {
    let raw: RawPost = serde_json::from_value(node.clone())
        .context("deserialize hashnode post summary")?;
    Ok(PostSummary {
        id: raw.id,
        title: raw.title,
        slug: raw.slug,
        brief: raw.brief.unwrap_or_default(),
        published_at: raw.published_at.unwrap_or_default(),
        read_time_minutes: raw.read_time_in_minutes.unwrap_or(0),
        cover_image_url: raw.cover_image.and_then(|c| c.url),
        tags: raw
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|t| Tag {
                name: t.name,
                slug: t.slug,
            })
            .collect(),
        author: Author {
            name: raw.author.name,
            username: raw.author.username.unwrap_or_default(),
            profile_picture: raw.author.profile_picture,
        },
    })
}

fn parse_post(node: &Value) -> Result<Post> {
    let raw: RawPost =
        serde_json::from_value(node.clone()).context("deserialize hashnode post")?;
    let content = raw.content.unwrap_or_default();
    Ok(Post {
        id: raw.id,
        title: raw.title,
        slug: raw.slug,
        subtitle: raw.subtitle,
        brief: raw.brief.unwrap_or_default(),
        published_at: raw.published_at.unwrap_or_default(),
        read_time_minutes: raw.read_time_in_minutes.unwrap_or(0),
        cover_image_url: raw.cover_image.and_then(|c| c.url),
        content_html: content.html.unwrap_or_default(),
        content_markdown: content.markdown.unwrap_or_default(),
        tags: raw
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|t| Tag {
                name: t.name,
                slug: t.slug,
            })
            .collect(),
        author: Author {
            name: raw.author.name,
            username: raw.author.username.unwrap_or_default(),
            profile_picture: raw.author.profile_picture,
        },
        seo: raw.seo.map(|s| Seo {
            title: s.title,
            description: s.description,
        }),
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawPost {
    id: String,
    title: String,
    slug: String,
    subtitle: Option<String>,
    brief: Option<String>,
    published_at: Option<String>,
    read_time_in_minutes: Option<u32>,
    cover_image: Option<RawCover>,
    #[serde(default)]
    tags: Option<Vec<RawTag>>,
    author: RawAuthor,
    content: Option<RawContent>,
    seo: Option<RawSeo>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawCover {
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawTag {
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RawAuthor {
    name: String,
    username: Option<String>,
    profile_picture: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct RawContent {
    markdown: Option<String>,
    html: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RawSeo {
    title: Option<String>,
    description: Option<String>,
}
