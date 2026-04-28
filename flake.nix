{
  description = "zuihitsu (随筆) — personal tech blog, Hashnode-backed, three deploy targets (K3s SSR · Cloudflare Pages static · Fly.io)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crate2nix = {
      url = "github:nix-community/crate2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # ishou — pleme-io design system, consumed from FlakeHub. The repo is
    # public (pleme-io-opensource/org.yaml) and publishes to FlakeHub via
    # substrate's reusable-flakehub.yml on every push to main. `fh:` lets
    # the flake input track rolling versions without pinning a commit.
    ishou.url = "https://flakehub.com/f/pleme-io/ishou/*.tar.gz";
    ishou.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, substrate, fenix, crate2nix, ishou, ... }:
    let
      # Existing K3s path: substrate's Leptos SSR+CSR dual-build.
      leptosFlake = (import "${substrate}/lib/leptos-build-flake.nix" {
        inherit nixpkgs substrate;
      }) {
        inherit self;
        name = "zuihitsu";
        ssrBinaryName = "zuihitsu";
        ssrFeatures = "ssr";
        ssrCargoArgs = "-p zuihitsu-app";
        csrFeatures = "hydrate";
        csrCargoArgs = "-p zuihitsu-app --profile wasm-release";
        wasmBindgenTarget = "web";
        optimizeLevel = 3;
        port = 3000;
        healthPort = 3000;
        staticAssets = ./public;
      };

      systems = [ "aarch64-darwin" "x86_64-linux" "aarch64-linux" ];

      # Cloudflare + sitegen + pangea apps. Scripted (not full Nix derivations)
      # so the repo ships today; will migrate to substrate recipes once those
      # land (substrate/lib/build/rust/{rust-static-site,cloudflare-worker}-flake.nix
      # and substrate/lib/build/web/cloudflare-pages-deploy.nix).
      mkExtras = system: let
        pkgs = import nixpkgs { inherit system; };
        fenixPkgs = fenix.packages.${system};
        rustToolchain = fenixPkgs.combine [
          fenixPkgs.latest.cargo
          fenixPkgs.latest.rustc
          fenixPkgs.targets.wasm32-unknown-unknown.latest.rust-std
          fenixPkgs.latest.clippy
          fenixPkgs.latest.rustfmt
        ];
        devTools = [
          rustToolchain
          pkgs.pkg-config
          pkgs.openssl
          pkgs.wasm-bindgen-cli
          pkgs.binaryen
          pkgs.nodejs_20
          pkgs.wrangler
          pkgs.bundler
          pkgs.ruby_3_3
          pkgs.opentofu
          # zuihitsu-dev requirements: cloudflared (tunnel app) is the only
          # extra binary not already covered. The dev daemon itself is a Rust
          # binary built via `cargo run` from inside `nix run .#dev`.
          pkgs.cloudflared
        ];
        binPath = pkgs.lib.makeBinPath devTools;

        mkApp = name: script: {
          type = "app";
          program = "${pkgs.writeShellScriptBin name ''
            set -euo pipefail
            export PATH=${binPath}:$PATH
            ${script}
          ''}/bin/${name}";
        };
        ishouBin = ishou.packages.${system}.default;
      in {
        # ── Design tokens (ishou → every target) ──────────────────────────
        tokens-css = mkApp "zuihitsu-tokens-css" ''
          ${ishouBin}/bin/ishou render --target css --out style/ishou.css
        '';
        tokens-tailwind = mkApp "zuihitsu-tokens-tailwind" ''
          ${ishouBin}/bin/ishou render --target tailwind --out tailwind.config.js
        '';
        tokens-hash = mkApp "zuihitsu-tokens-hash" ''
          ${ishouBin}/bin/ishou hash
        '';
        tokens-sync = mkApp "zuihitsu-tokens-sync" ''
          ${ishouBin}/bin/ishou render --target css --out style/ishou.css
          ${ishouBin}/bin/ishou render --target svg --out public/favicon.svg
          echo "synced ishou tokens → style/ishou.css + public/favicon.svg"
        '';

        # ── Cloudflare Pages path ─────────────────────────────────────────
        generate = mkApp "zuihitsu-generate" ''
          # Ensure ishou tokens are fresh before generation.
          ${ishouBin}/bin/ishou render --target css --out style/ishou.css
          cargo build --release --features sitegen --bin zuihitsu-sitegen -p zuihitsu-app
          ./target/release/zuihitsu-sitegen "''${1:-dist}"
          echo "wrote dist/ — upload with: nix run .#pages-deploy"
        '';
        pages-deploy = mkApp "zuihitsu-pages-deploy" ''
          [[ -d dist ]] || { echo "no dist/ — run: nix run .#generate"; exit 1; }
          wrangler pages deploy dist --project-name=zuihitsu --branch=main
        '';

        # ── Cloudflare Worker path ────────────────────────────────────────
        #
        # `worker-build` lives on crates.io, not in nixpkgs, so we `cargo
        # install` it on first use into $HOME/.cargo/bin. The subsequent
        # `worker-build --release` invocation needs that path to be visible,
        # so prepend it to PATH before anything else.
        #
        # Version pin: the `worker` crate line we use (0.5) is compatible
        # with worker-build ^0.1. worker-build 0.8+ requires worker >= 0.8.
        # Bump the pin below the day we upgrade `worker` in Cargo.toml.
        worker-build = mkApp "zuihitsu-worker-build" ''
          export PATH="$HOME/.cargo/bin:$PATH"
          cd crates/zuihitsu-worker
          if ! command -v worker-build >/dev/null 2>&1; then
            cargo install -q --locked "worker-build@^0.1"
          fi
          worker-build --release
        '';
        worker-deploy = mkApp "zuihitsu-worker-deploy" ''
          # CLOUDFLARE_ACCOUNT_ID — wrangler reads it from env; wrangler.toml
          # intentionally omits `account_id =` so this is the single source.
          # Decrypt CLOUDFLARE_API_TOKEN ahead of time from the nix sops file:
          #   export CLOUDFLARE_API_TOKEN=$(cd ../nix && \
          #     sops -d --extract '["cloudflare"]["api-token"]' secrets.yaml)
          : "''${CLOUDFLARE_API_TOKEN?must be set (see header)}"
          : "''${CLOUDFLARE_ACCOUNT_ID:=97d01f39d2967f21320f41bf71249ed1}"
          export CLOUDFLARE_ACCOUNT_ID
          [[ -d crates/zuihitsu-worker/build ]] || {
            echo "no build/ — run: nix run .#worker-build" >&2; exit 1;
          }
          wrangler deploy --config crates/zuihitsu-worker/wrangler.toml
        '';

        # Pangea IaC moved to pangea-architectures/workspaces/cloudflare-pleme.
        # Run pangea commands from that workspace instead:
        #   cd ../pangea-architectures/workspaces/cloudflare-pleme
        #   bundle exec pangea {synth,plan,apply,destroy} quero_cloud.rb

        # ── Freescape fit check (arch-synthesizer) ────────────────────────
        freescape-check = mkApp "zuihitsu-freescape-check" ''
          echo "Expected: Cloudflare always-free tier"
          echo "Pages: static, unlimited bw"
          echo "Workers: 100k requests/day"
          echo "DNS + TLS: free"
          echo "R2: 10GB + 0 egress"
          echo ""
          echo "TODO: wire arch-synthesizer FreescapeCheck against"
          echo "      pangea-architectures/workspaces/cloudflare-pleme/quero_cloud.rb"
          echo "      once pangea emits a WasmWorkloadDecl sidecar."
        '';

        # ── Dev loop (zuihitsu-dev) ───────────────────────────────────────
        # Each wrapper is a single `exec` of either the Rust dev binary
        # (built on demand by cargo) or an external tool like wrangler /
        # cloudflared. No shell logic beyond the exec — keeps within the
        # pleme-io 3-line glue policy and centralises behaviour in
        # crates/zuihitsu-dev.
        dev = mkApp "zuihitsu-dev-watch" ''
          exec cargo run --profile dev-fast -p zuihitsu-dev -- daemon "$@"
        '';
        fetch = mkApp "zuihitsu-dev-fetch" ''
          exec cargo run --profile dev-fast -p zuihitsu-dev -- fetch "$@"
        '';
        draft = mkApp "zuihitsu-dev-draft" ''
          exec cargo run --profile dev-fast -p zuihitsu-dev -- draft "$@"
        '';
        worker-test = mkApp "zuihitsu-dev-worker-test" ''
          exec cargo run --profile dev-fast -p zuihitsu-dev -- worker-test "$@"
        '';
        # Local worker via wrangler dev (miniflare). Worker bundle must already
        # exist at crates/zuihitsu-worker/build/ — run `nix run .#worker-build`
        # first.
        worker-dev = mkApp "zuihitsu-worker-dev" ''
          cd crates/zuihitsu-worker
          exec wrangler dev
        '';
        # Cloudflared quick-tunnel for end-to-end webhook smoke tests against
        # a real Hashnode payload signed by Hashnode itself.
        tunnel = mkApp "zuihitsu-tunnel" ''
          exec cloudflared tunnel --url http://localhost:8787
        '';
        # Prod-parity preview: render with inlined CSS via nix run .#generate
        # first, then serve via wrangler pages dev (Pages routing rules,
        # _headers, _redirects, etc. — closer to deployed shape than
        # `nix run .#dev`).
        preview = mkApp "zuihitsu-preview" ''
          exec wrangler pages dev "''${1:-dist}"
        '';
      };
    in
      leptosFlake // {
        apps = nixpkgs.lib.genAttrs systems (system:
          (leptosFlake.apps.${system} or {}) // (mkExtras system)
        );
      };
}
