# zuihitsu/pangea

Pangea workspace for the Cloudflare-native deploy path.

## What this renders

`zuihitsu.rb` declares:

| Resource | Purpose |
|---|---|
| `cloudflare_zone pleme_io` | DNS zone (if not already managed elsewhere) |
| `cloudflare_pages_project zuihitsu` | Static site project for the blog |
| `cloudflare_pages_domain zuihitsu_apex` | Attach `blog.pleme.io` to Pages |
| `cloudflare_dns_record blog_cname` | CNAME → `zuihitsu.pages.dev` (proxied) |
| `cloudflare_workers_script zuihitsu_webhook` | Webhook receiver (WASM uploaded via wrangler) |
| `cloudflare_workers_route zuihitsu_webhook_route` | `webhook.blog.pleme.io/*` → worker |
| `cloudflare_dns_record webhook_cname` | Worker AAAA record |

## Ownership boundary

Terraform (via pangea) owns project/domain/route/DNS. Content is uploaded
out-of-band:
- **Pages dist/**: `wrangler pages deploy dist/` after `nix run .#generate`
- **Worker WASM**: `wrangler deploy --config crates/zuihitsu-worker/wrangler.toml`

This split mirrors how lilitu does Cloudflare deploys — resource shape is
declarative, bytes are pushed by the deploy tool.

## Freescape target

`pangea.yml` declares `freescape.provider: cloudflare`, `profile: always-free`.
Before apply, `nix run .#freescape-check` runs `FreescapeCheck` from
arch-synthesizer against the `WasmWorkloadDecl` derived from these resources.
Overage is a build-break.

## Render / apply

```bash
nix run .#pangea-render      # zuihitsu.rb → terraform.tf.json
nix run .#pangea-plan        # terraform plan
nix run .#pangea-apply       # terraform apply (gated by freescape-check)
```
