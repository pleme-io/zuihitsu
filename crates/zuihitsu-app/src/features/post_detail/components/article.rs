use leptos::prelude::*;

use crate::entities::Post;
use crate::features::post_detail::components::prose::Prose;
use crate::features::posts::components::tag_chip::TagChip;
use crate::infra::utils::format::{format_short_date, reading_time_label};

#[component]
pub fn Article(post: Post) -> impl IntoView {
    let date = format_short_date(&post.published_at);
    let read = reading_time_label(post.read_time_minutes);
    let tags = post.tags.clone();
    view! {
        <article class="z-article">
            <header class="z-article-header">
                <h1 class="z-article-title">{post.title.clone()}</h1>
                {post.subtitle.clone().map(|s| view! {
                    <p class="z-article-subtitle">{s}</p>
                })}
                <div class="z-article-meta">
                    <time>{date}</time>
                    <span class="z-card-dot">"·"</span>
                    <span>{read}</span>
                </div>
            </header>
            {post.cover_image_url.clone().map(|url| view! {
                <img class="z-article-cover" src=url alt="" loading="eager"/>
            })}
            <Prose html=post.content_html.clone()/>
            <footer class="z-article-footer">
                <div class="z-card-tags">
                    {tags.into_iter().map(|t| view! { <TagChip tag=t/> }).collect_view()}
                </div>
            </footer>
        </article>
    }
}
