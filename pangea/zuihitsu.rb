# frozen_string_literal: true
#
# zuihitsu — Cloudflare Pages (static) + Worker (webhook) synthesis.
#
# Freescape-native deploy: Pages serves the pre-rendered blog, a tiny Worker
# verifies Hashnode webhooks and triggers a GitHub Actions rebuild. DNS is
# pointed at the Pages custom domain.
#
# Rendered via pangea-cloudflare → Terraform JSON → tofu apply.

require 'pangea'
require 'pangea-cloudflare'

template :zuihitsu do
  provider :cloudflare do
    api_token var(:cloudflare_api_token)
  end

  account = var(:cloudflare_account_id)
  zone_name = var(:zone_name, default: 'pleme.io')
  site_host = var(:site_host, default: 'blog.pleme.io')
  webhook_host = var(:webhook_host, default: 'webhook.blog.pleme.io')

  zone = cloudflare_zone(
    :pleme_io,
    {
      account: { id: account },
      name: zone_name,
      type: 'full'
    }
  )

  # ── Pages (static site) ──────────────────────────────────────────
  # The site bundle is produced by `nix run .#generate` in the zuihitsu
  # repo and uploaded to Pages via GHA (wrangler pages deploy dist/).
  # Terraform owns the PROJECT + DOMAIN; uploads are out-of-band.
  pages = cloudflare_pages_project(
    :zuihitsu,
    {
      account_id: account,
      name: 'zuihitsu',
      production_branch: 'main',
      build_config: {
        build_command: '',
        destination_dir: 'dist',
        root_dir: ''
      },
      deployment_configs: {
        production: {
          env_vars: {
            ZUIHITSU_SITE_URL: { type: 'plain_text', value: "https://#{site_host}" }
          }
        }
      }
    }
  )

  cloudflare_pages_domain(
    :zuihitsu_apex,
    {
      account_id: account,
      project_name: pages.name,
      name: site_host
    }
  )

  cloudflare_dns_record(
    :blog_cname,
    {
      zone_id: zone.id,
      name: site_host,
      type: 'CNAME',
      content: "#{pages.subdomain}",
      ttl: 1,        # automatic
      proxied: true
    }
  )

  # ── Worker (Hashnode webhook → GHA dispatch) ─────────────────────
  # Content is uploaded separately by the zuihitsu worker-deploy flow
  # (cargo worker-build → wrangler deploy). This resource reserves the
  # script name + bindings; Terraform won't push WASM bytes here.
  webhook = cloudflare_workers_script(
    :zuihitsu_webhook,
    {
      account_id: account,
      script_name: 'zuihitsu-webhook',
      main_module: 'shim.mjs',
      compatibility_date: '2025-01-01',
      observability: { enabled: true, head_sampling_rate: 1 }
    }
  )

  cloudflare_workers_route(
    :zuihitsu_webhook_route,
    {
      zone_id: zone.id,
      pattern: "#{webhook_host}/*",
      script: webhook.script_name
    }
  )

  cloudflare_dns_record(
    :webhook_cname,
    {
      zone_id: zone.id,
      name: webhook_host,
      type: 'AAAA',
      content: '100::',   # Cloudflare Workers placeholder address
      ttl: 1,
      proxied: true
    }
  )
end
