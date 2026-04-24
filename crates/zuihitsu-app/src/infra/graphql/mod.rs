pub mod queries;

// Hashnode GraphQL client: native-only (reqwest + tokio), never shipped in
// the hydrate wasm bundle. Both SSR and the sitegen binary use it.
#[cfg(any(feature = "ssr", feature = "sitegen"))]
pub mod client;
