//! Page shell — wraps pages with header + footer + max-width column.

use leptos::prelude::*;

use super::footer::Footer;
use super::header::Header;

#[component]
pub fn PageShell(children: ChildrenFn) -> impl IntoView {
    view! {
        <div class="z-shell">
            <Header/>
            <main class="z-main">{children()}</main>
            <Footer/>
        </div>
    }
}
