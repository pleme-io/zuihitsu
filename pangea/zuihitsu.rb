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
# The template body is a single call to
# Pangea::Architectures::CloudflareHeadlessBlog — the same pattern is reused
# for any future Pages+Worker blog. site_record_id/webhook_record_id pin the
# existing zuihitsu Terraform state IDs so the refactor is a no-op for tofu.
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
require 'pangea/architectures'
require 'digest'

template :zuihitsu do
  template_fingerprint = Digest::SHA256.hexdigest(File.read(__FILE__))

  api_token    = ENV.fetch('CLOUDFLARE_API_TOKEN')   { raise 'CLOUDFLARE_API_TOKEN not set' }
  account      = ENV.fetch('CLOUDFLARE_ACCOUNT_ID')  { raise 'CLOUDFLARE_ACCOUNT_ID not set' }
  zone_name    = ENV.fetch('ZONE_NAME',    'pleme.io')
  site_host    = ENV.fetch('SITE_HOST',    'blog.pleme.io')
  webhook_host = ENV.fetch('WEBHOOK_HOST', 'webhook.blog.pleme.io')

  provider :cloudflare, api_token: api_token

  Pangea::Architectures::CloudflareHeadlessBlog.build(self, {
    account_id: account,
    zone_name: zone_name,
    slug: 'zuihitsu',
    site_host: site_host,
    webhook_host: webhook_host,
    env_vars: {
      ZUIHITSU_SITE_URL: { type: 'plain_text', value: "https://#{site_host}" }
    },
    # Pin the pre-architecture Terraform state IDs so this refactor does not
    # drop-and-recreate the two DNS records.
    site_record_id: :blog_cname,
    webhook_record_id: :webhook_cname
  })

  output :pangea_fingerprint do
    value template_fingerprint
    description "SHA256 of #{File.basename(__FILE__)} — tamper detection"
  end
end
