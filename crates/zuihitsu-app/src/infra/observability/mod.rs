//! Observability primitives. Kept minimal — tracing bootstrap lives at the
//! crate entry points (main.rs for SSR, lib.rs for hydrate).

pub fn service_name() -> &'static str {
    "zuihitsu"
}
