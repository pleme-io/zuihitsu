{
  description = "zuihitsu (随筆) — personal tech blog, Leptos SSR rendering Hashnode content";

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
  };

  outputs = { self, nixpkgs, substrate, fenix, crate2nix, ... }:
    (import "${substrate}/lib/leptos-build-flake.nix" {
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
}
