use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::widgets::PageShell;

#[component]
pub fn AboutPage() -> impl IntoView {
    view! {
        <PageShell>
            <Title text="About · zuihitsu"/>
            <Meta name="description" content="About zuihitsu and its author."/>
            <article class="z-prose">
                <h1>"About"</h1>
                <p>
                    "zuihitsu (" <i>"随筆"</i> ") is the classical Japanese literary genre for \
                    personal essays — fragmentary musings written as the brush follows thought. \
                    This is mine."
                </p>
                <p>
                    "I write about Rust, distributed systems, infrastructure-as-code, and the \
                    platforms I build. Posts are authored in Hashnode and rendered by a \
                    Leptos SSR app in the pleme-io workspace."
                </p>
                <p>
                    <a href="https://github.com/drzln" rel="noopener">"GitHub"</a>
                    " · "
                    <a href="/rss.xml">"RSS"</a>
                </p>
            </article>
        </PageShell>
    }
}
