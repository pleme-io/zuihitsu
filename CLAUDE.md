# zuihitsu (随筆)

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


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

# Dev loop — see "Dev loop" below for what each does
nix run .#dev                    # daemon: watch sources, hot-reload at :3000
nix run .#fetch                  # invalidate Hashnode disk cache + re-warm
nix run .#draft -- <slug>        # scaffold drafts/<slug>.md
nix run .#worker-dev             # wrangler dev --local for the worker
nix run .#worker-test -- --secret <s>  # POST signed mock webhook to local worker
nix run .#tunnel                 # cloudflared quick-tunnel to local worker
nix run .#preview                # wrangler pages dev (prod-parity smoke)

nix develop                      # fenix + cargo + wasm-bindgen + wrangler + ruby + tofu
```

## Dev loop

The fast inner loop is the `zuihitsu-dev` daemon (`crates/zuihitsu-dev/`).
Single Rust binary, four subcommands; the flake apps are 1-2 line `exec`
wrappers per the pleme-io no-shell policy.

What you get from `nix run .#dev`:

| You edit | What happens | Latency |
|----------|--------------|---------|
| `style/*.css` | daemon copies to dist/, pushes WS `css` event, browser swaps `<link>` href | <100ms, no flash, no scroll loss |
| `public/*` | daemon copies to dist/, pushes WS `reload` | ~50ms |
| `crates/zuihitsu-app/src/**` | cargo build (dev-fast profile) + sitegen + WS `reload` | ~1–3s after first build |
| `drafts/*.md` | sitegen `--only home,posts,tags,feeds` + WS `reload` | <2s (cargo skipped) |
| Hashnode publishes a post | poller (every 30s by default) detects via summary hash, invalidates cache, re-runs sitegen | ~30s after publish |
| Build fails | WS `error` → full-screen overlay in the browser with cargo / sitegen output | immediate |

Behind it:
- **Disk cache** for Hashnode GraphQL responses, keyed by
  `blake3(query body)`. Defaults to `.cache/hashnode/`. `zuihitsu-dev fetch`
  invalidates + re-warms; `ZUIHITSU_HASHNODE_OFFLINE=1` errors on miss
  instead of falling through to the network.
- **Linked CSS in dev** via `ZUIHITSU_DEV_LINKED_CSS=1` (set automatically by
  the daemon). Production keeps the inline-CSS one-round-trip path.
- **`drafts/` (gitignored)** — local-only markdown files with YAML
  frontmatter that merge into the Hashnode post list. Format documented in
  `crates/zuihitsu-app/src/infra/draft.rs`. Sitegen reads them via
  `--drafts <dir>`; production GHA never passes the flag.
- **`--only` flag on sitegen** — `home,about,not_found,posts,tags,feeds,assets,all`.
  Static targets (`about`, `not_found`, `assets`) skip Hashnode entirely so
  they work offline.
- **Custom `dev-fast` cargo profile** — `incremental=true`, `codegen-units=256`,
  `debug="line-tables-only"`, `split-debuginfo="unpacked"`. Cuts the sitegen
  rebuild from ~8s to ~1–2s after the first build.

Reusable substrate recipe at
`substrate/lib/build/web/static-site-dev-loop.nix` exposes
`mkDevApp`/`mkFetchApp`/`mkDraftApp`/`mkWorkerTestApp`/`mkWorkerDevApp`/`mkTunnelApp`/`mkPreviewApp`
factories — copy `crates/zuihitsu-dev/` into the next blog (rename to
`<name>-dev`), consume the recipe, you're done.

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
- `substrate/lib/build/web/static-site-dev-loop.nix` ← landed; consumed by zuihitsu's flake inline today
- `substrate/lib/infra/cloudflare-headless-blog.nix`
- `substrate/lib/service/headless-blog-sdlc.nix`

Skill: `blackmatter-pleme/skills/cloudflare-headless-blog/`.

Until those land, this repo's `flake.nix` is bespoke (scripted apps).
The migration is mechanical and will land as a follow-up commit.

## Status — what's done, what's next

**Done (2026-04-25 — dev-loop milestone):**

- `zuihitsu-dev` daemon (`crates/zuihitsu-dev/`) — single Rust binary, four
  subcommands (`daemon`, `fetch`, `draft`, `worker-test`). 16 unit tests.
- Hashnode response disk cache (`infra/graphql/client.rs`), blake3-keyed,
  with `ZUIHITSU_HASHNODE_OFFLINE=1` for strict mode.
- Drafts loader (`infra/draft.rs`) — YAML frontmatter + markdown.
- Sitegen CLI: `--only` and `--drafts` flags, parallel post fetches via
  `try_join_all`. Static targets (about / not_found / assets) bypass
  Hashnode entirely so they work offline.
- `ZUIHITSU_DEV_LINKED_CSS=1` toggle in `static_render::shell()` — link
  tags in dev (so the daemon can swap stylesheets without a Rust rebuild),
  inlined in production.
- `[profile.dev-fast]` in `Cargo.toml` — incremental, no-LTO, 256
  codegen-units, line-tables-only debug, unpacked split-debuginfo.
- Seven flake apps: `dev`, `fetch`, `draft`, `worker-test`, `worker-dev`,
  `tunnel`, `preview` — each a 1-2 line `exec` wrapper per the pleme-io
  no-shell policy.
- Substrate recipe: `substrate/lib/build/web/static-site-dev-loop.nix`
  with `mkDevApp` / `mkFetchApp` / `mkDraftApp` / `mkWorkerTestApp` /
  `mkWorkerDevApp` / `mkTunnelApp` / `mkPreviewApp` / `mkAllApps`
  factories.

**Next (in rough priority order):**

1. **Wire the GHA `repository_dispatch` workflow.** The worker fires a
   `zuihitsu-rebuild` event but no workflow listens for it yet. Without
   this, real Hashnode publishes don't actually rebuild prod. Add
   `.github/workflows/rebuild.yml` that runs
   `nix run .#generate && nix run .#pages-deploy` on
   `repository_dispatch: zuihitsu-rebuild`.
2. **Package `zuihitsu-dev` as a Nix derivation.** Today `nix run .#dev`
   pays a cargo build on first invocation (~30-60s cold). Adding it to
   the crate2nix build set or wrapping with `rustPlatform.buildRustPackage`
   makes the first launch instant. Same for the other six apps.
3. **Extract `zuihitsu-dev` → generic `pleme-static-dev` crate.** Once a
   second blog (novaskyn?) needs the same loop, lift the daemon into its
   own crates.io-published library and have the substrate recipe build
   that crate via crate2nix instead of relying on a per-repo copy.
4. **Fix substrate's deprecated `pkgs.nodePackages.npm` reference**
   (`substrate/lib/build/rust/leptos-build.nix:245`). Currently breaks
   `nix develop` against current nixpkgs. Replace with `pkgs.nodejs_20`
   (which already provides `npm`).
5. **Hashnode publish CLI** (`zuihitsu-dev publish drafts/<slug>.md`).
   Posts a draft to Hashnode via the GraphQL mutation API, gated on a
   `HASHNODE_PAT` env var. Closes the local-draft loop end-to-end.
6. **Per-render-fn dependency map.** Today any change under
   `crates/zuihitsu-app/src/` triggers a full sitegen via `--only all`.
   Splitting `static_render/mod.rs` into `static_render/{shell,home,post,
   tag,about,not_found}.rs` plus a small file → target table in
   `daemon/watch.rs` would let template edits map to the minimal
   `--only home` / `--only posts` / etc.
7. **Source-link the build error overlay.** `daemon/server.rs` pushes the
   raw cargo output; if we parse the `path:line:col` prefix and emit
   `vscode://` / `cursor://` links, clicking the overlay jumps to the
   offending line.
