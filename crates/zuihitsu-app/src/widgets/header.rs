use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="z-header">
            <A href="/" attr:class="z-brand">
                <span class="z-brand-kanji">"随筆"</span>
                <span class="z-brand-text">"zuihitsu"</span>
            </A>
            <nav class="z-nav">
                <A href="/" attr:class="z-nav-link">"Essays"</A>
                <A href="/about" attr:class="z-nav-link">"About"</A>
                <a class="z-nav-link" href="/rss.xml" aria-label="RSS feed">"RSS"</a>
            </nav>
        </header>
    }
}
