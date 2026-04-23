use leptos::prelude::*;
use leptos_meta::Title;

use crate::widgets::PageShell;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        if let Some(resp) = use_context::<leptos_axum::ResponseOptions>() {
            resp.set_status(http::StatusCode::NOT_FOUND);
        }
    }
    view! {
        <PageShell>
            <Title text="Not found · zuihitsu"/>
            <div class="z-empty">
                <h1>"404"</h1>
                <p>"The essay you sought is not here."</p>
                <p><a href="/">"Return to the index"</a></p>
            </div>
        </PageShell>
    }
}
