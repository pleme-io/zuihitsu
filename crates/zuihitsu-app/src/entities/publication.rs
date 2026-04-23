use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Publication {
    pub id: String,
    pub title: String,
    pub display_title: Option<String>,
    pub about_html: Option<String>,
    pub og_image_url: Option<String>,
    pub favicon_url: Option<String>,
}
