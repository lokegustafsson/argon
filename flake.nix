{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix";
      inputs.rust-overlay.follows = "rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, cargo2nix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ cargo2nix.overlays.default ];
          config.allowUnfree = false;
        };
        lib = nixpkgs.lib;

        patched-simd-json-src = let
          gzipped = pkgs.fetchurl {
            url = "https://crates.io/api/v1/crates/simd-json/0.10.3/download";
            sha256 = "sha256-o9CBXn/w8fBeCdSwKfhtijMPCrFbNbKHNvN1gyX1nhQ=";
          };
        in pkgs.runCommand "patched-simd-json-src" { } ''
          cp ${gzipped} src.tar.gz
          tar xvf src.tar.gz
          mv simd-json-0.10.3 $out
          cd $out
          patch -s --strip=1 < ${./patches/avx2_deser.patch}
          patch -s --strip=1 < ${./patches/charutils.patch}
          patch -s --strip=1 < ${./patches/serde_se_pp.patch}
          ${pkgs.rustfmt}/bin/rustfmt $out/src/charutils.rs
          ${pkgs.rustfmt}/bin/rustfmt $out/src/avx2/deser.rs
          ${pkgs.rustfmt}/bin/rustfmt $out/src/serde/se/pp.rs
        '';

        rust = import ./rust.nix {
          inherit lib pkgs;
          extra-overrides = { mkNativeDep, mkEnvDep, mkRpath, mkOverride, p }: [
            (mkEnvDep "argon" { TEST_CASE_DIR = ./testcases; })
            (mkOverride "simd-json" (old: {
              src = patched-simd-json-src;
              unpackPhase = ''
                echo src = $src
                cp -r $src/* .
                chmod -R +w .
              '';
            }))
          ];
        };

        large-file-json = pkgs.fetchurl {
          url =
            "https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json";
          sha256 = "sha256-T8HlLE5gn+vQXXWiTIS8aVf6TSz7DV++u6xlC9x+2MA=";
        };

        argonBin = (rust.argon { }).bin;
        argonTest = pkgs.rustBuilder.runTests rust.argon { };
      in {
        devShells.default = rust.workspaceShell {
          packages = let p = pkgs;
          in [
            cargo2nix.outputs.packages.${system}.cargo2nix
            p.cargo-flamegraph
            p.gron
            p.rust-bin.stable.latest.clippy
            p.rust-bin.stable.latest.default
            p.rust-bin.stable.latest.rust-analyzer
          ];
          LD_LIBRARY_PATH = let p = pkgs; in lib.makeLibraryPath [ ];
          LARGE_FILE_JSON = large-file-json;
          shellHook = ''
            BASE=$(basename "$(pwd)")
            FLAKE="$(pwd)/flake.nix"
            if [[ ("$BASE" != "argon") || (! -f "$FLAKE") ]]; then
                echo "Must be run from the argon source tree root!"
                exit 1
            fi
            unset BASE FLAKE

            export TEST_CASE_DIR="$(pwd)/testcases"

            git rev-parse --is-inside-work-tree > /dev/null && \
            export CARGO_TARGET_DIR="$HOME/cargo-target-dir$(git rev-parse --show-toplevel)"
          '';
        };

        packages = {
          inherit patched-simd-json-src;
          default = argonBin;
          unit-test = argonTest;
        };

        apps = builtins.mapAttrs (name: value: {
          type = "app";
          program =
            let app = pkgs.writeShellApplication (value // { inherit name; });
            in "${app}/bin/${name}";
        }) (import ./apps.nix {
          inherit argonBin pkgs large-file-json patched-simd-json-src;
          cargo2nix = cargo2nix.outputs.packages.${system}.cargo2nix;
        });
      });
}
