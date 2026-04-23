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
    # ishou — pleme-io design system. Currently pinned to a local path since
    # the GitHub repo creation goes through pangea-github + repo-forge (see
    # blackmatter-code memory feedback on IaC-first repo creation). Swap to
    # `github:pleme-io/ishou` once the repo lands.
    ishou = {
      url = "path:../ishou";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
          pkgs.nodePackages.wrangler
          pkgs.bundler
          pkgs.ruby_3_3
          pkgs.opentofu
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
        worker-build = mkApp "zuihitsu-worker-build" ''
          cd crates/zuihitsu-worker
          if ! command -v worker-build >/dev/null 2>&1; then
            cargo install -q --locked worker-build
          fi
          worker-build --release
        '';
        worker-deploy = mkApp "zuihitsu-worker-deploy" ''
          wrangler deploy --config crates/zuihitsu-worker/wrangler.toml
        '';

        # ── Pangea (Cloudflare IaC) ───────────────────────────────────────
        pangea-install = mkApp "zuihitsu-pangea-install" ''
          cd pangea && bundle install --path .bundle
        '';
        pangea-render = mkApp "zuihitsu-pangea-render" ''
          cd pangea && bundle exec pangea render zuihitsu.rb -o ../terraform/main.tf.json
        '';
        pangea-plan = mkApp "zuihitsu-pangea-plan" ''
          cd pangea && bundle exec pangea plan zuihitsu.rb
        '';
        pangea-apply = mkApp "zuihitsu-pangea-apply" ''
          cd pangea && bundle exec pangea apply zuihitsu.rb
        '';

        # ── Freescape fit check (arch-synthesizer) ────────────────────────
        freescape-check = mkApp "zuihitsu-freescape-check" ''
          echo "Expected: Cloudflare always-free tier"
          echo "Pages: static, unlimited bw"
          echo "Workers: 100k requests/day"
          echo "DNS + TLS: free"
          echo "R2: 10GB + 0 egress"
          echo ""
          echo "TODO: wire arch-synthesizer FreescapeCheck against pangea/zuihitsu.rb"
          echo "      once pangea emits a WasmWorkloadDecl sidecar."
        '';
      };
    in
      leptosFlake // {
        apps = nixpkgs.lib.genAttrs systems (system:
          (leptosFlake.apps.${system} or {}) // (mkExtras system)
        );
      };
}
