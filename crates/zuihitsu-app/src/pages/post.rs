use leptos::either::EitherOf3;
use leptos::prelude::*;
use leptos_meta::{Meta, Title};
use leptos_router::hooks::use_params_map;

use crate::features::post_detail::components::article::Article;
use crate::shared::server_fns::fetch_post;
use crate::widgets::PageShell;

#[component]
pub fn PostPage() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.with(|p| p.get("slug").unwrap_or_default());

    let post = Resource::new(slug, |slug| async move { fetch_post(slug).await });

    view! {
        <PageShell>
            <Suspense fallback=move || view! { <div class="z-spinner" aria-label="Loading"/> }>
                {move || Suspend::new(async move {
                    match post.await {
                        Ok(Some(p)) => {
                            let title = format!("{} · zuihitsu", p.title);
                            let desc = p.seo.as_ref()
                                .and_then(|s| s.description.clone())
                                .unwrap_or_else(|| p.brief.clone());
                            EitherOf3::A(view! {
                                <Title text=title/>
                                <Meta name="description" content=desc.clone()/>
                                <Meta property="og:title" content=p.title.clone()/>
                                <Meta property="og:description" content=desc/>
                                {p.cover_image_url.clone().map(|url| view! {
                                    <Meta property="og:image" content=url/>
                                })}
                                <Article post=p/>
                            })
                        }
                        Ok(None) => EitherOf3::B(view! {
                            <Title text="Not found · zuihitsu"/>
                            <div class="z-empty">"Post not found."</div>
                        }),
                        Err(e) => EitherOf3::C(view! {
                            <Title text="Error · zuihitsu"/>
                            <div class="z-empty">{format!("Failed to load post: {e}")}</div>
                        }),
                    }
                })}
            </Suspense>
        </PageShell>
    }
}
