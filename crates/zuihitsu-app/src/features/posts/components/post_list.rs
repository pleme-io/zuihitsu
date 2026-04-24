use leptos::either::Either;
use leptos::prelude::*;

use crate::features::posts::components::post_card::PostCard;
use crate::shared::server_fns::{fetch_posts, fetch_posts_by_tag};

#[component]
pub fn PostList() -> impl IntoView {
    let posts = Resource::new(
        || (),
        |()| async move { fetch_posts(None, Some(20)).await },
    );
    view! {
        <Suspense fallback=move || view! { <div class="z-spinner" aria-label="Loading"/> }>
            {move || Suspend::new(async move {
                match posts.await {
                    Ok(page) if page.posts.is_empty() => Either::Left(view! {
                        <p class="z-empty">"No essays published yet."</p>
                    }),
                    Ok(page) => Either::Right(Either::Left(view! {
                        <section class="z-post-list">
                            {page.posts.into_iter()
                                .map(|p| view! { <PostCard post=p/> })
                                .collect_view()}
                        </section>
                    })),
                    Err(e) => Either::Right(Either::Right(view! {
                        <p class="z-empty">{format!("Failed to load posts: {e}")}</p>
                    })),
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn PostListByTag(tag_slug: String) -> impl IntoView {
    let slug_for_resource = tag_slug.clone();
    let posts = Resource::new(
        move || slug_for_resource.clone(),
        |slug| async move { fetch_posts_by_tag(slug, None, Some(20)).await },
    );
    view! {
        <Suspense fallback=move || view! { <div class="z-spinner" aria-label="Loading"/> }>
            {move || Suspend::new(async move {
                match posts.await {
                    Ok(page) if page.posts.is_empty() => Either::Left(view! {
                        <p class="z-empty">"Nothing tagged yet."</p>
                    }),
                    Ok(page) => Either::Right(Either::Left(view! {
                        <section class="z-post-list">
                            {page.posts.into_iter()
                                .map(|p| view! { <PostCard post=p/> })
                                .collect_view()}
                        </section>
                    })),
                    Err(e) => Either::Right(Either::Right(view! {
                        <p class="z-empty">{format!("Failed to load posts: {e}")}</p>
                    })),
                }
            })}
        </Suspense>
    }
}
