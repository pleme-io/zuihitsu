//! Theme provider.
//!
//! The Nord palette is baked into `style/main.css` as CSS custom properties,
//! so the document already has a theme before hydrate — no FOUC. This provider
//! exposes a `ThemeState` context for future runtime toggling (light/high-contrast)
//! and mirrors the lilitu-web theme provider API.

use leptos::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy)]
pub struct ThemeState {
    pub mode: ReadSignal<ThemeMode>,
    pub set_mode: WriteSignal<ThemeMode>,
}

pub fn use_theme() -> ThemeState {
    use_context::<ThemeState>().expect("ThemeState not provided — wrap the app in ThemeProvider")
}

#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
    let (mode, set_mode) = signal(ThemeMode::Dark);
    provide_context(ThemeState { mode, set_mode });
    children()
}
