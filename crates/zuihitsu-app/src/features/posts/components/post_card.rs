use leptos::prelude::*;
use leptos_router::components::A;

use crate::entities::PostSummary;
use crate::features::posts::components::tag_chip::TagChip;
use crate::infra::utils::format::{format_short_date, reading_time_label};

#[component]
pub fn PostCard(post: PostSummary) -> impl IntoView {
    let href = format!("/posts/{}", post.slug);
    let date = format_short_date(&post.published_at);
    let read = reading_time_label(post.read_time_minutes);
    let tags = post.tags.clone();
    let cover = post.cover_image_url.clone();
    view! {
        <article class="z-card">
            {cover.map(|url| view! {
                <A href=href.clone() attr:class="z-card-cover-link">
                    <img class="z-card-cover" src=url alt="" loading="lazy"/>
                </A>
            })}
            <A href=href.clone() attr:class="z-card-title-link">
                <h2 class="z-card-title">{post.title.clone()}</h2>
            </A>
            <p class="z-card-brief">{post.brief.clone()}</p>
            <div class="z-card-meta">
                <time>{date}</time>
                <span class="z-card-dot">"·"</span>
                <span>{read}</span>
            </div>
            <div class="z-card-tags">
                {tags.into_iter().map(|t| view! { <TagChip tag=t/> }).collect_view()}
            </div>
        </article>
    }
}
