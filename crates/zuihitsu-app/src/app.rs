//! Root application component.
//!
//! Wires up providers (theme, meta, PWA) and composes the router. Mirrors the
//! provider-stack pattern from lilitu-web.

use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, MetaTags, Title, provide_meta_context};

use crate::providers::pwa::PwaProvider;
use crate::providers::theme::ThemeProvider;
use crate::router::AppRouter;

/// Server-side HTML shell. Called once per request by leptos_axum to render
/// the document scaffold before the `<App>` tree is streamed in.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#2e3440"/>
                <meta name="color-scheme" content="dark"/>
                <link rel="manifest" href="/manifest.json"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <link rel="alternate" type="application/rss+xml" title="zuihitsu" href="/rss.xml"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <HashedStylesheet id="leptos" options=options.clone()/>
                <style>{include_str!("../../../style/ishou.css")}</style>
                <style>{include_str!("../../../style/main.css")}</style>
                <MetaTags/>
            </head>
            <body>
                <ZuihitsuApp/>
            </body>
        </html>
    }
}

#[component]
pub fn ZuihitsuApp() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="zuihitsu · 随筆"/>
        <ThemeProvider>
            <PwaProvider>
                <ErrorBoundary fallback=|errors| {
                    view! {
                        <div class="z-error">
                            <h2>"Something went wrong"</h2>
                            <ul>
                                {move || errors
                                    .get()
                                    .into_iter()
                                    .map(|(_, e)| view! { <li>{e.to_string()}</li> })
                                    .collect::<Vec<_>>()
                                }
                            </ul>
                        </div>
                    }
                }>
                    <Suspense fallback=move || view! { <div class="z-spinner" aria-label="Loading"/> }>
                        <AppRouter/>
                    </Suspense>
                </ErrorBoundary>
            </PwaProvider>
        </ThemeProvider>
    }
}
