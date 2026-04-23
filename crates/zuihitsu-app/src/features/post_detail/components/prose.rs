use leptos::prelude::*;

/// Renders trusted HTML supplied by Hashnode. Hashnode sanitizes author input
/// server-side, so we render `inner_html` directly — no client sanitization.
#[component]
pub fn Prose(html: String) -> impl IntoView {
    view! {
        <div class="z-prose" inner_html=html></div>
    }
}
