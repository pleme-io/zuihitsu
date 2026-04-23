//! PWA provider — tracks online/offline state and surfaces an offline banner.
//!
//! Service worker registration itself happens in `shared::sw` during hydrate.

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct PwaState {
    pub is_online: ReadSignal<bool>,
}

pub fn use_pwa() -> PwaState {
    use_context::<PwaState>().expect("PwaState not provided")
}

#[component]
pub fn PwaProvider(children: Children) -> impl IntoView {
    let (is_online, set_online) = signal(true);
    provide_context(PwaState { is_online });

    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        if let Some(win) = web_sys::window() {
            set_online.set(win.navigator().on_line());
            let set_on = set_online;
            let online_cb = Closure::<dyn FnMut()>::new(move || set_on.set(true));
            let set_off = set_online;
            let offline_cb = Closure::<dyn FnMut()>::new(move || set_off.set(false));
            let _ = win.add_event_listener_with_callback(
                "online",
                online_cb.as_ref().unchecked_ref(),
            );
            let _ = win.add_event_listener_with_callback(
                "offline",
                offline_cb.as_ref().unchecked_ref(),
            );
            online_cb.forget();
            offline_cb.forget();
        }
    }

    let _ = set_online;

    view! {
        <>
            <Show when=move || !is_online.get() fallback=|| ()>
                <div class="z-offline-banner" role="status">
                    "You are offline — showing cached content"
                </div>
            </Show>
            {children()}
        </>
    }
}
