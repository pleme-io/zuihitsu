use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="z-footer">
            <span class="z-footer-text">
                "© " <Year/> " · written in "
                <a href="https://hashnode.com" rel="noopener">"Hashnode"</a>
                " · served from Rust"
            </span>
            <a class="z-footer-link" href="https://github.com/pleme-io/zuihitsu" rel="noopener">
                "source"
            </a>
        </footer>
    }
}

#[component]
fn Year() -> impl IntoView {
    // Rendered once at build, fine for a footer copyright.
    let year: i32 = {
        #[cfg(feature = "ssr")]
        { chrono::Utc::now().date_naive().format("%Y").to_string().parse().unwrap_or(2026) }
        #[cfg(not(feature = "ssr"))]
        { 2026 }
    };
    view! { {year} }
}
