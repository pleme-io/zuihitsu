# zuihitsu (随筆)

Personal tech blog. Leptos SSR rendering Hashnode-authored content.

## Architecture

- **Content:** authored in Hashnode (publication `drzln.hashnode.dev`). Fetched
  server-side via GraphQL at request time. Browser never talks to Hashnode.
- **Server:** `zuihitsu` axum binary renders HTML, serves the hydrate bundle,
  generates `/sitemap.xml` + `/rss.xml`, and exposes `/healthz`/`/readyz`.
- **Client:** leptos hydrate bundle (cdylib WASM) takes over after initial paint.
- **Build:** substrate `leptos-build-flake.nix` → dual SSR binary + CSR WASM
  bundle + Docker image (single layered image, axum serves both).
- **Deploy:** Helm chart at `chart/zuihitsu/`, FluxCD HelmRelease template at
  `k8s-manifests/infrastructure/zuihitsu/helmrelease.yaml` (copy into
  `pleme-io/k8s` per cluster).

## Layout (Feature-Sliced Design)

Mirrors `lilitu-web`'s FSD layering. Higher-numbered layers can only import
from lower-numbered layers.

```
crates/zuihitsu-app/src/
├── lib.rs                 entry, hydrate export
├── main.rs                axum server (feature = "ssr")
├── app.rs                 root + shell(), provider tree
├── router.rs              leptos_router routing table
├── providers/             theme, pwa  (layer 1)
├── entities/              Post, Tag, Publication, Author, PostPage (layer 2)
├── infra/                 graphql, markdown, feed, utils (layer 3)
├── widgets/               Header, Footer, PageShell (layer 4)
├── features/              posts, post_detail, newsletter (layer 5)
├── pages/                 Home, Post, Tag, About, NotFound (layer 6)
└── shared/                hooks, stores, sw, server_fns (cross-cutting leaf)
```

## Features

| Feature | Purpose |
|---------|---------|
| `ssr`   | axum + tokio + reqwest + pulldown_cmark + tracing-subscriber |
| `hydrate` | wasm-bindgen + gloo-* + web-sys + tracing-web + console_error_panic_hook |

Dependencies are `optional = true` and pulled in exclusively by the
matching feature so a wasm build never pulls tokio etc. and the native
binary never pulls wasm-bindgen.

## Build

```bash
nix develop                     # fenix + trunk + wasm-bindgen + tailwindcss
nix build                       # combined SSR + CSR + docker image
nix run .#dockerImage           # load image into local daemon
```

In the dev shell:

```bash
cargo build --release --features ssr --bin zuihitsu -p zuihitsu-app
cargo build --release --features hydrate --target wasm32-unknown-unknown -p zuihitsu-app
```

## Runtime configuration

| Env | Default | Purpose |
|-----|---------|---------|
| `LEPTOS_SITE_ADDR` | `0.0.0.0:3000` | bind address |
| `LEPTOS_SITE_ROOT` | `/static` | path where CSR WASM bundle lives on disk |
| `ZUIHITSU_HASHNODE_HOST` | `drzln.hashnode.dev` | Hashnode publication |
| `RUST_LOG` | `info,zuihitsu=debug` | tracing filter |

## Adding a new page

1. Create `crates/zuihitsu-app/src/pages/<name>.rs` with a `#[component]`
   exporting the page.
2. Re-export from `src/pages/mod.rs`.
3. Add a `<Route>` in `src/router.rs`.
4. If the page needs data, add a `#[server]` fn in `src/shared/server_fns.rs`
   that delegates to an SSR-only client in `src/infra/`.

## Adding a new feature slice

`features/<slice>/{mod,components,hooks,machines}.rs`. Features may import
from `entities/`, `infra/`, `shared/`, `widgets/`, but NOT from other
features or from `pages/`/`app.rs`.

## GraphQL

Queries live as `&'static str` in `src/infra/graphql/queries.rs`. The client
at `src/infra/graphql/client.rs` (`Hashnode::from_env`) is SSR-only — all
browser fetches go through `#[server]` functions in
`src/shared/server_fns.rs`. No CORS, no token in the hydrate bundle.

## SEO

- Every page sets `<Title>` + `<Meta name="description">` via `leptos_meta`.
- Post pages additionally emit OpenGraph tags from the Hashnode `seo` field.
- `/sitemap.xml` paginates the full publication.
- `/rss.xml` emits the latest 20 posts.
- SSR renders the full DOM and meta tags before hydrate — crawlers see
  content, not an empty shell.

## PWA

- `public/manifest.json` installable metadata.
- `public/sw.js` Workbox 7 — network-first HTML, cache-first images/fonts,
  SWR for `/pkg/*`.
- `public/version.json` polled every 5m by `shared/hooks/use_version_check`
  to prompt the user to reload on new deploys.
- Service worker is registered during `hydrate()` in `lib.rs`.

## Styling

No Tailwind build step. `style/main.css` defines the Nord palette as CSS
custom properties (matching irodori's `SemanticColors::nord()`) and ships
inlined into every SSR response via `include_str!` in `app::shell()`.
Component styling is class-based (`.z-*`) with small inline `style=""` only
for per-instance values.

## Conventions (pleme-io)

- Edition 2024, Rust 1.89.0+, MIT, public GitHub.
- `[lints.clippy] pedantic = "warn"` at workspace level.
- Release profile: `codegen-units=1`, `lto=true`, `opt-level="z"`, `strip=true`.
- Commit directly to `main` and push. No PRs. No Claude co-author attribution.
