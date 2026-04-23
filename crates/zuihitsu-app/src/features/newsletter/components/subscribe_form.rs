//! Placeholder subscribe form. Hashnode's `subscribeToNewsletter` mutation
//! can be wired into a `#[server]` function later; until then, this component
//! just surfaces the RSS link.

use leptos::prelude::*;

#[component]
pub fn SubscribeForm() -> impl IntoView {
    view! {
        <aside class="z-subscribe">
            <h3 class="z-subscribe-title">"Subscribe"</h3>
            <p class="z-subscribe-body">
                "Follow new posts via "
                <a href="/rss.xml">"RSS"</a>
                "."
            </p>
        </aside>
    }
}
