use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use leptos_router::hooks::use_params_map;

use crate::features::posts::components::post_list::PostListByTag;
use crate::widgets::PageShell;

#[component]
pub fn TagPage() -> impl IntoView {
    let params = use_params_map();
    let tag = Signal::derive(move || params.with(|p| p.get("tag").unwrap_or_default()));

    view! {
        <PageShell>
            {move || {
                let t = tag.get();
                let title = format!("#{t} · zuihitsu");
                let desc = format!("Posts tagged #{t}");
                view! {
                    <Title text=title/>
                    <Meta name="description" content=desc/>
                    <h1 class="z-tag-heading">"#"{t.clone()}</h1>
                    <PostListByTag tag_slug=t/>
                }
            }}
        </PageShell>
    }
}
