use serde::{Deserialize, Serialize};

use super::tag::Tag;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostSummary {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub brief: String,
    pub published_at: String,
    pub read_time_minutes: u32,
    pub cover_image_url: Option<String>,
    pub tags: Vec<Tag>,
    pub author: Author,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub subtitle: Option<String>,
    pub brief: String,
    pub published_at: String,
    pub read_time_minutes: u32,
    pub cover_image_url: Option<String>,
    pub content_html: String,
    pub content_markdown: String,
    pub tags: Vec<Tag>,
    pub author: Author,
    pub seo: Option<Seo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub username: String,
    pub profile_picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Seo {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PostPage {
    pub posts: Vec<PostSummary>,
    pub next_cursor: Option<String>,
    pub has_next: bool,
}
