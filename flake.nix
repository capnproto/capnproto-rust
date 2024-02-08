{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    nixpkgs-21.url = "github:NixOS/nixpkgs/nixos-21.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    inputs@{ self
    , flake-utils
    , nixpkgs
    , rust-overlay
    , nixpkgs-21
    , crane
    , advisory-db
    , ...
    }:
    flake-utils.lib.eachSystem [ flake-utils.lib.system.x86_64-linux ] (system:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };

      rust-custom-toolchain = (pkgs.rust-bin.stable.latest.default.override {
        extensions = [
          "rust-src"
          "rustfmt"
          "llvm-tools-preview"
          "rust-analyzer-preview"
        ];
      });

    in
    rec {
      devShell = (pkgs.mkShell.override { stdenv = pkgs.llvmPackages_15.stdenv; }) {
        buildInputs = with pkgs; [
          openssl
          pkg-config
        ];

        nativeBuildInputs = with pkgs; [
          # get current rust toolchain defaults (this includes clippy and rustfmt)
          rust-custom-toolchain

          cargo-edit

          capnproto

          cmake

          ninja
        ];

        # fetch with cli instead of native
        CARGO_NET_GIT_FETCH_WITH_CLI = "true";
        RUST_BACKTRACE = 1;
      };

      default = { };

      checks =
        let
          craneLib =
            (inputs.crane.mkLib pkgs).overrideToolchain rust-custom-toolchain;
          src = ./.;
          pname = "capnp-checks";
          version = "0.1.0";
          stdenv = pkgs.llvmPackages_15.stdenv;

          cargoArtifacts = craneLib.buildDepsOnly {
            inherit src pname version stdenv;
            buildInputs = with pkgs; [ openssl pkg-config ];
          };
          build-tests = craneLib.buildPackage {
            inherit cargoArtifacts src pname version stdenv;
            buildInputs = with pkgs; [ pkg-config capnproto cmake openssl ];
          };
        in
        {
          inherit build-tests;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          capnp-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src stdenv version;
            pname = "${pname}-clippy";
            cargoClippyExtraArgs = "-- --deny warnings";

            buildInputs = with pkgs; [ openssl pkg-config capnproto cmake];
          };

          # Check formatting
          capnp-fmt = craneLib.cargoFmt {
            inherit src stdenv version;
            pname = "${pname}-fmt";
          };

          # Audit dependencies
          capnp-audit = craneLib.cargoAudit {
            inherit src stdenv version;
            pname = "${pname}-audit";
            advisory-db = inputs.advisory-db;
            cargoAuditExtraArgs = "--ignore RUSTSEC-2020-0071";
          };

          # Run tests with cargo-nextest
          capnp-nextest = craneLib.cargoNextest {
            inherit cargoArtifacts src stdenv version;
            pname = "${pname}-nextest";
            partitions = 1;
            partitionType = "count";

            buildInputs = with pkgs; [ openssl pkg-config capnproto cmake];
          };
        };

    });
}
