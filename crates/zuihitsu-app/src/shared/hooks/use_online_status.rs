//! Reactive `navigator.onLine` status. Always reports `true` on SSR.

use leptos::prelude::*;

#[must_use]
pub fn use_online_status() -> ReadSignal<bool> {
    let (online, set_online) = signal(true);
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        if let Some(win) = web_sys::window() {
            set_online.set(win.navigator().on_line());
            let s_on = set_online;
            let on_cb = Closure::<dyn FnMut()>::new(move || s_on.set(true));
            let s_off = set_online;
            let off_cb = Closure::<dyn FnMut()>::new(move || s_off.set(false));
            let _ = win.add_event_listener_with_callback("online", on_cb.as_ref().unchecked_ref());
            let _ =
                win.add_event_listener_with_callback("offline", off_cb.as_ref().unchecked_ref());
            on_cb.forget();
            off_cb.forget();
        }
    }
    let _ = set_online;
    online
}
