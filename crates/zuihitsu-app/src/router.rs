//! Central routing table. Every route handler lives under `pages/`.

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::pages::{AboutPage, HomePage, NotFoundPage, PostPage, TagPage};

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFoundPage/> }>
                <Route path=path!("/") view=HomePage/>
                <Route path=path!("/posts/:slug") view=PostPage/>
                <Route path=path!("/tags/:tag") view=TagPage/>
                <Route path=path!("/about") view=AboutPage/>
            </Routes>
        </Router>
    }
}
