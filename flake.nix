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
                ++ [ ./patches/avx2_deser.patch ./patches/charutils.patch ];
            }))
          ];
        };
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
          pwd
          ls
          patch -s --strip=1 < ${./patches/avx2_deser.patch}
          patch -s --strip=1 < ${./patches/charutils.patch}
        '';
        large-file-json = pkgs.fetchurl {
          url =
            "https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json";
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
          LARGE_FILE_JSON = large-file-json;
          PATCHED_SIMD_JSON_SRC = patched-simd-json-src;
        };

        packages.default = rust.argon;

        apps = {
          bench = {
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
              printf "@ $(pwd)\n"
              set -v
              time argon ${large-file-json} > large-file.argon
              time argon --ungron large-file.argon > large-file.json
            '');
          };
          compare = {
            type = "app";
            program = builtins.toString (pkgs.writeShellScript "compare" ''
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

              printf "\ngron --ungron:"
              time gron --ungron gron.result > gron.json
              printf "\nargon --ungron:"
              time argon --ungron argon.result > argon.json
            '');
          };
          test = {
            type = "app";
            program = builtins.toString (pkgs.writeShellScript "bench" ''
              export PATH=${
                lib.strings.makeBinPath [
                  pkgs.bash
                  pkgs.coreutils
                  pkgs.diffutils
                  pkgs.gron
                  pkgs.wget
                  rust.argon
                ]
              }
              printf "\nComparing on escaping.json.."
              G1=$(sha256sum <(gron ${./test/escaping.json}))
              A1=$(sha256sum <(argon ${./test/escaping.json}))
              diff <(echo $G1) <(echo $A1)

              printf "\nComparing on large-file.json.."
              G2=$(sha256sum <(gron ${large-file-json}))
              A2=$(sha256sum <(argon ${large-file-json}))
              diff <(echo $G2) <(echo $A2)
            '');
          };
          flamegraph = {
            type = "app";
            program = builtins.toString (pkgs.writeShellScript "flamegraph" ''
              export PATH=${
                lib.strings.makeBinPath [
                  pkgs.bash
                  pkgs.coreutils
                  pkgs.cargo-flamegraph
                  rust.argon
                ]
              }
              flamegraph -- argon ${large-file-json} > /dev/null
            '');
          };
        };
      });
}
