use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::features::posts::components::post_list::PostList;
use crate::features::newsletter::components::subscribe_form::SubscribeForm;
use crate::widgets::PageShell;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <PageShell>
            <Title text="zuihitsu · 随筆 — personal tech essays"/>
            <Meta name="description" content="Rust, infrastructure, and systems essays by drzln."/>
            <section class="z-hero">
                <h1 class="z-hero-title">"随筆"</h1>
                <p class="z-hero-subtitle">
                    "Essays on Rust, infrastructure, and what I'm building."
                </p>
            </section>
            <PostList/>
            <SubscribeForm/>
        </PageShell>
    }
}
