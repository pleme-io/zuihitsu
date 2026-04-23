use leptos::prelude::*;
use leptos_router::components::A;

use crate::entities::Tag;

#[component]
pub fn TagChip(tag: Tag) -> impl IntoView {
    let href = format!("/tags/{}", tag.slug);
    view! {
        <A href=href attr:class="z-tag-chip">"#"{tag.name}</A>
    }
}
