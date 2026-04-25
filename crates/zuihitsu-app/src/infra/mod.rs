pub mod graphql;
pub mod markdown;
pub mod observability;
pub mod utils;

#[cfg(any(feature = "ssr", feature = "sitegen"))]
pub mod feed;

// Local-draft loader (dev loop). Pulled in alongside feed because both depend
// on the SSR-side reqwest/pulldown-cmark stack.
#[cfg(any(feature = "ssr", feature = "sitegen"))]
pub mod draft;
