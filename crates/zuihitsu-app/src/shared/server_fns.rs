//! Server functions — the only surface the browser calls into. Each one is
//! feature-gated so only its body ships to SSR; the browser only sees the
//! thin RPC stub.

use leptos::prelude::*;

use crate::entities::{Post, PostPage, Tag};

#[server(FetchPosts, "/api")]
pub async fn fetch_posts(
    cursor: Option<String>,
    limit: Option<i32>,
) -> Result<PostPage, ServerFnError> {
    use crate::infra::graphql::client::Hashnode;
    let client = Hashnode::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    client
        .list_posts(cursor.as_deref(), limit.unwrap_or(10))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(FetchPost, "/api")]
pub async fn fetch_post(slug: String) -> Result<Option<Post>, ServerFnError> {
    use crate::infra::graphql::client::Hashnode;
    let client = Hashnode::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    client
        .get_post(&slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(FetchPostsByTag, "/api")]
pub async fn fetch_posts_by_tag(
    tag_slug: String,
    cursor: Option<String>,
    limit: Option<i32>,
) -> Result<PostPage, ServerFnError> {
    use crate::infra::graphql::client::Hashnode;
    let client = Hashnode::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    client
        .list_posts_by_tag(&tag_slug, cursor.as_deref(), limit.unwrap_or(10))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(FetchTags, "/api")]
pub async fn fetch_tags() -> Result<Vec<Tag>, ServerFnError> {
    use crate::infra::graphql::client::Hashnode;
    let client = Hashnode::from_env().map_err(|e| ServerFnError::new(e.to_string()))?;
    client
        .list_tags()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
