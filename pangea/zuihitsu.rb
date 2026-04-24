# frozen_string_literal: true
#
# zuihitsu — Cloudflare Pages (static) + Worker (webhook) synthesis.
#
# Freescape-native deploy: Pages serves the pre-rendered blog, a tiny Worker
# verifies Hashnode webhooks and triggers a GitHub Actions rebuild. DNS is
# pointed at the Pages custom domain.
#
# Rendered via pangea-cloudflare → Terraform JSON → tofu apply.
#
# Configuration resolves from:
#   - ENV overrides (CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID, ZONE_NAME,
#     SITE_HOST, WEBHOOK_HOST)
#   - workspace pangea.yml / root pangea.yml (via Pangea::WorkspaceConfig)
# This matches the canonical pleme-io template pattern (see pangea-core
# CLAUDE.md → Template Pattern). `var(...)` is deliberately not used here —
# it emits `${var.foo}` terraform refs, but we resolve at synthesis time
# against ENV + workspace config instead.

require 'pangea-core'
require 'pangea-cloudflare'
require 'digest'

template :zuihitsu do
  template_fingerprint = Digest::SHA256.hexdigest(File.read(__FILE__))

  api_token    = ENV.fetch('CLOUDFLARE_API_TOKEN')   { raise 'CLOUDFLARE_API_TOKEN not set' }
  account      = ENV.fetch('CLOUDFLARE_ACCOUNT_ID')  { raise 'CLOUDFLARE_ACCOUNT_ID not set' }
  zone_name    = ENV.fetch('ZONE_NAME',    'pleme.io')
  site_host    = ENV.fetch('SITE_HOST',    'blog.pleme.io')
  webhook_host = ENV.fetch('WEBHOOK_HOST', 'webhook.blog.pleme.io')

  extend(Pangea::Resources::Cloudflare) unless respond_to?(:cloudflare_zone)

  provider :cloudflare, api_token: api_token

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
      project_name: 'zuihitsu',
      name: site_host
    }
  )

  cloudflare_dns_record(
    :blog_cname,
    {
      zone_id: zone.id,
      name: site_host,
      type: 'CNAME',
      content: pages.subdomain,
      ttl: 1,
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
      content: '100::',
      ttl: 1,
      proxied: true
    }
  )

  output :pangea_fingerprint do
    value template_fingerprint
    description "SHA256 of #{File.basename(__FILE__)} — tamper detection"
  end
end
