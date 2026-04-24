pub mod entities;
pub mod infra;
pub mod static_render;

// Leptos view-tree modules — only compiled when targeting SSR or hydrate.
// The sitegen binary produces pure-string HTML via `static_render` and does
// not depend on the Leptos runtime at all.
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod app;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod features;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod pages;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod providers;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod router;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod shared;
#[cfg(any(feature = "ssr", feature = "hydrate"))]
pub mod widgets;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::ZuihitsuApp;
    console_error_panic_hook::set_once();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(tracing_web::MakeConsoleWriter)
        .without_time();
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    tracing_subscriber::registry().with(fmt_layer).init();

    shared::sw::register_service_worker();
    leptos::mount::hydrate_body(ZuihitsuApp);
}
