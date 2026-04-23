//! Polls `/version.json` every 5 minutes and flips a signal when the version
//! string changes — used to prompt the user to reload for a fresh build.
//! No-op on SSR.

use leptos::prelude::*;

#[must_use]
pub fn use_version_check() -> ReadSignal<bool> {
    let (stale, set_stale) = signal(false);
    #[cfg(feature = "hydrate")]
    {
        use gloo_net::http::Request;
        use gloo_timers::future::TimeoutFuture;
        use wasm_bindgen_futures::spawn_local;

        spawn_local(async move {
            let mut known: Option<String> = None;
            loop {
                if let Ok(resp) = Request::get("/version.json").send().await
                    && let Ok(body) = resp.text().await
                    && let Ok(v) = serde_json::from_str::<serde_json::Value>(&body)
                    && let Some(ver) = v.get("version").and_then(|x| x.as_str())
                {
                    match &known {
                        None => known = Some(ver.to_string()),
                        Some(prev) if prev != ver => {
                            set_stale.set(true);
                            break;
                        }
                        _ => {}
                    }
                }
                TimeoutFuture::new(5 * 60 * 1000).await;
            }
        });
    }
    let _ = set_stale;
    stale
}
