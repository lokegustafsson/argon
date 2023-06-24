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
        rust = import ./rust.nix {
          inherit lib pkgs;
          extra-overrides = { mkNativeDep, mkEnvDep, mkRpath, mkOverride, p }: [
            (mkNativeDep "argon" [ ])
            (mkOverride "simd-json" (old: {
              patches = (if old ? patches then old.patches else [ ])
                ++ [ ./patches/simd-json-keep-escaped.patch ];
            }))
          ];
        };
        large-file-json = pkgs.fetchurl {
          url = "https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json";
          sha256 = "sha256-T8HlLE5gn+vQXXWiTIS8aVf6TSz7DV++u6xlC9x+2MA=";
        };
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
        };

        packages.default = rust.argon;

        apps.bench = {
          type = "app";
          program = builtins.toString (pkgs.writeShellScript "bench" ''
            export PATH=${
              lib.strings.makeBinPath [
                pkgs.bash
                pkgs.coreutils
                pkgs.gron
                pkgs.wget
                rust.argon
              ]
            }
            cd ''${TMPDIR:-/tmp}
            printf "@ $(pwd)"
            printf "\ngron:"
            time gron ${large-file-json} > gron.result
            printf "\nargon:"
            time argon ${large-file-json} > argon.result
          '');
        };
      });
}
