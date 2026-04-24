# zuihitsu (随筆)

Personal tech blog. Hashnode-authored content rendered by a Rust pipeline
with three interchangeable deploy targets.

## Three targets, one codebase

| Target | Status | Substrate | Cost | Trigger |
|---|---|---|---|---|
| **Cloudflare Pages + Worker** | primary (`deploy.yaml` active) | static HTML + `worker-rs` | $0 (always-free) | Hashnode webhook → GHA |
| **K3s SSR** | fallback | `leptos_axum` + Helm + FluxCD | cluster capacity | FluxCD reconcile |
| **Fly.io** | fallback | same Docker image as K3s | ~$2–5/mo | `fly deploy` |

Which target is active is declared in `deploy.yaml → render_targets.*.active`.

## Architecture (Cloudflare target — primary)

```
Hashnode (CMS)
     │
     │ 1. webhook on publish
     ▼
Cloudflare Worker (crates/zuihitsu-worker)
  HMAC verify → POST /dispatches (github)
     │
     ▼
GitHub Actions
  nix run .#generate → dist/
  wrangler pages deploy dist/ --project-name zuihitsu
     │
     ▼
Cloudflare Pages (blog.quero.cloud)
  static HTML, served from edge
```

Static generation is a pure Rust tokio binary (`zuihitsu-sitegen`, feature-
gated on `sitegen`) that fetches everything from Hashnode and emits
`dist/{index.html, about/, posts/<slug>/, tags/<slug>/, sitemap.xml, rss.xml}`
plus copies of `public/` and `style/main.css`.

## Layout (Feature-Sliced Design, crates workspace)

```
zuihitsu/
├── crates/
│   ├── zuihitsu-app/                 Leptos app + sitegen binary + axum SSR
│   │   └── src/
│   │       ├── lib.rs · main.rs · app.rs · router.rs
│   │       ├── bin/sitegen.rs        static site generator (feature sitegen)
│   │       ├── providers/            theme, pwa
│   │       ├── entities/             Post, Tag, Publication, Author
│   │       ├── infra/                graphql, feed, markdown, utils
│   │       ├── widgets/              Header, Footer, PageShell
│   │       ├── features/             posts, post_detail, newsletter
│   │       ├── pages/                Home, Post, Tag, About, NotFound
│   │       ├── shared/               server_fns, hooks, sw
│   │       └── static_render/        pure-string HTML for sitegen
│   └── zuihitsu-worker/              Cloudflare Worker (crate-type cdylib)
│                                     (IaC lives in pangea-architectures —
│                                      see "Pangea IaC" below)
├── chart/zuihitsu/                   Helm chart (K3s path)
├── k8s-manifests/                    FluxCD HelmRelease template
├── style/main.css                    Nord palette, inlined by sitegen + SSR
├── public/                           manifest.json, sw.js, version.json, favicon
├── flake.nix                         Nix entry — see "Apps" below
├── deploy.yaml                       forge product config (which target is active)
└── CLAUDE.md                         this file
```

## Features

| Feature | Purpose | Included in bin |
|---------|---------|-----------------|
| `ssr`     | axum + leptos_axum + tokio | `zuihitsu` |
| `hydrate` | wasm-bindgen + web-sys + gloo-* + tracing-web | cdylib |
| `sitegen` | tokio + reqwest + pulldown-cmark (no axum) | `zuihitsu-sitegen` |

The worker crate is independent — it uses `worker-rs` which is wasm32 + no tokio,
so it has to live in its own crate (`zuihitsu-worker`) to avoid feature collisions.

## Nix apps

```bash
# Cloudflare path (primary)
nix run .#generate               # fetch Hashnode, write dist/
nix run .#pages-deploy           # wrangler pages deploy dist/
nix run .#worker-build           # compile crates/zuihitsu-worker/ → .wasm + shim
nix run .#worker-deploy          # wrangler deploy
nix run .#freescape-check        # validate fit against Cloudflare free tier

# Pangea IaC (quero.cloud zone + Pages + Worker + Porkbun NS delegation)
# lives in pangea-architectures:
#   cd ../pangea-architectures/workspaces/cloudflare-pleme
#   bundle exec pangea {synth,plan,apply,destroy} quero_cloud.rb

# K3s path
nix build                        # combined SSR binary + CSR WASM + docker image
nix run .#default                # run SSR binary locally

# Dev
nix develop                      # fenix + cargo + wasm-bindgen + wrangler + ruby + tofu
```

## Runtime configuration

| Env | Default | Used by |
|-----|---------|---------|
| `ZUIHITSU_HASHNODE_HOST` | `drzln.hashnode.dev` | sitegen, SSR |
| `ZUIHITSU_SITE_URL` | `https://blog.quero.cloud` | sitegen, SSR, sitemap, RSS |
| `LEPTOS_SITE_ADDR` | `0.0.0.0:3000` | SSR |
| `LEPTOS_SITE_ROOT` | `/static` | SSR |
| `RUST_LOG` | `info,zuihitsu=debug` | both |

### Cloudflare Worker secrets (wrangler secret put ...)

| Secret | Purpose |
|--------|---------|
| `WEBHOOK_SECRET` | Hashnode HMAC signing secret (prefix `whsec_`) |
| `GITHUB_TOKEN` | fine-grained PAT, repo `pleme-io/zuihitsu`, `contents:read` + `actions:write` |
| `GITHUB_REPO` | `pleme-io/zuihitsu` (var, not secret) |

### Pangea IaC

Zone + Pages + Worker are declared in
`pangea-architectures/workspaces/cloudflare-pleme/quero_cloud.rb`, a thin
template that calls `Pangea::Architectures::CloudflareHeadlessBlog`. It
shares state, credentials, and the import workflow with the other three
Cloudflare-account templates (lilitu, novaskyn, tunnel) in that workspace.

| Var | Source |
|-----|--------|
| `CLOUDFLARE_API_TOKEN` | `sops -d --extract '["cloudflare"]["api-token"]' ../nix/secrets.yaml` |
| `CLOUDFLARE_ACCOUNT_ID` | ENV, default baked into template (`97d01f39d2967f21320f41bf71249ed1`) |
| `SITE_HOST` | ENV, default `blog.quero.cloud` |
| `WEBHOOK_HOST` | ENV, default `webhook.blog.quero.cloud` |

## Freescape

The Cloudflare target is declared as `freescape.provider: cloudflare`,
`profile: always-free` on the `cloudflare-pages` render target in
`deploy.yaml`. The `arch-synthesizer` crate has `FreescapeBudget` +
`FreescapeCheck` — wire them up against the Pangea output with
`nix run .#freescape-check`. Overage = build break.

## Adding a new page

1. Add `render_xxx()` in `crates/zuihitsu-app/src/static_render/mod.rs`.
2. Call it from `crates/zuihitsu-app/src/bin/sitegen.rs` and write to `dist/xxx/index.html`.
3. Add a matching Leptos page at `src/pages/xxx.rs` + a `<Route>` in `router.rs`
   so the SSR path mirrors. Duplicate templates are intentional — the static
   and SSR code paths should be readable independently.

## GraphQL / Hashnode

Queries live as `&'static str` in `src/infra/graphql/queries.rs`. The client
at `src/infra/graphql/client.rs` is SSR+sitegen only. Browser never talks
to Hashnode — SSR uses `#[server]` functions, sitegen uses the client
directly at build time.

## SEO

- SSR + sitegen both emit full `<title>` + `<meta name="description">` +
  OpenGraph tags before the crawler sees a single byte of JS.
- `/sitemap.xml` paginates the full publication.
- `/rss.xml` emits the latest 20 posts.
- Cloudflare Pages serves these as static files; K3s SSR axum serves them live.

## PWA

- `public/manifest.json` — installable metadata.
- `public/sw.js` — Workbox 7, network-first HTML, cache-first images/fonts.
- `public/version.json` — polled every 5m by `shared/hooks/use_version_check`.
- Service worker registered in `hydrate()` (K3s path only; Pages is static).

## Conventions (pleme-io)

- Edition 2024, Rust 1.89.0+, MIT.
- `[lints.clippy] pedantic = "warn"` at workspace level.
- Release profile: `codegen-units=1`, `lto=true`, `opt-level="z"`, `strip=true`.
- Commit directly to `main` and push. No PRs. No Claude co-author attribution.

## Substrate / reuse

The patterns here (Rust sitegen feeding Cloudflare Pages + a Worker for
webhooks + Pangea Cloudflare IaC) are in the process of being extracted
into substrate recipes:

- `substrate/lib/build/rust/rust-static-site-flake.nix`
- `substrate/lib/build/rust/cloudflare-worker-flake.nix`
- `substrate/lib/build/web/cloudflare-pages-deploy.nix`
- `substrate/lib/infra/cloudflare-headless-blog.nix`
- `substrate/lib/service/headless-blog-sdlc.nix`

Skill: `blackmatter-pleme/skills/cloudflare-headless-blog/`.

Until those land, this repo's `flake.nix` is bespoke (scripted apps).
The migration is mechanical and will land as a follow-up commit.
