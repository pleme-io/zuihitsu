//! Service worker registration. Fires once during hydrate.

#[cfg(feature = "hydrate")]
pub fn register_service_worker() {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(sw_container) = js_sys::Reflect::get(&window, &"navigator".into())
        .and_then(|nav| js_sys::Reflect::get(&nav, &"serviceWorker".into()))
    else {
        return;
    };
    if sw_container.is_undefined() {
        return;
    }
    let container: web_sys::ServiceWorkerContainer = sw_container.unchecked_into();
    let _ = container.register("/sw.js");
    tracing::info!("service worker registration initiated");
}
