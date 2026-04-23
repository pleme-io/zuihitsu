#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use axum::{Router, routing::get};
    use leptos::config::get_configuration;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use tower_http::compression::CompressionLayer;
    use tower_http::trace::TraceLayer;
    use zuihitsu_app::app::{ZuihitsuApp, shell};
    use zuihitsu_app::infra::feed::{sitemap_xml, rss_xml};

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,zuihitsu=debug".into()),
        )
        .json()
        .init();

    let conf = get_configuration(None).map_err(|e| anyhow::anyhow!("{e}"))?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(ZuihitsuApp);

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/readyz", get(|| async { "ok" }))
        .route("/sitemap.xml", get(sitemap_xml))
        .route("/rss.xml", get(rss_xml))
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || shell(opts.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<LeptosOptions, _>(
            shell,
        ))
        .layer(CompressionLayer::new().br(true).gzip(true))
        .layer(TraceLayer::new_for_http())
        .with_state(leptos_options);

    tracing::info!(%addr, "zuihitsu listening");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "ssr"))]
fn main() {}
