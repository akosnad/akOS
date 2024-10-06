{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane/v0.19.0";
  };

  outputs = { nixpkgs, flake-utils, fenix, crane, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            fenix.overlays.default
          ];

          pkgs = import nixpkgs { inherit system overlays; };

          rustToolchain = (pkgs.fenix.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-xrDB7Mc1uD80lzXu/ysFlfWmtAXkxaHSwcu26zbX/0U=";
          });

          craneLib = crane.mkLib pkgs;
          craneToolchain = craneLib.overrideToolchain rustToolchain;
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;

            strictDeps = false;
            doCheck = false;
            dontPatchELF = true;

            cargoExtraArgs = "-Zbindeps --target x86_64-unknown-none";

            nativeBuildInputs = [ ];
          };

          cargoArtifacts = craneToolchain.buildDepsOnly commonArgs;

          crate = craneToolchain.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
        in
        {
          devShells.default = with pkgs; mkShell {
            buildInputs = [
              openssl
              pkg-config
              qemu

              rustToolchain

              cargo-generate
            ];
          };

          packages.default = crate;
        });
}
