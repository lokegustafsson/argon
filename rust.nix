{ lib, pkgs, extra-overrides, use-mold ? true }:
let
  rustPkgs = pkgs.rustBuilder.makePackageSet {
    rustVersion = "latest";
    packageFun = import ./Cargo.nix;

    # Enable avx2
    hostPlatformCpu = "haswell";

    packageOverrides = p:
      let
        mkNativeDep = cratename: extra-deps:
          p.rustBuilder.rustLib.makeOverride {
            name = cratename;
            overrideAttrs = drv: {
              propagatedBuildInputs = (drv.propagatedBuildInputs or [ ])
                ++ extra-deps;
            };
          };
        mkEnvDep = cratename: env-deps:
          (p.rustBuilder.rustLib.makeOverride {
            name = cratename;
            overrideAttrs = drv: env-deps;
          });
        mkRpath = cratename: libs:
          p.rustBuilder.rustLib.makeOverride {
            name = cratename;
            overrideAttrs = drv: {
              postFixup = let libPath = lib.makeLibraryPath libs;
              in ''
                patchelf --add-rpath "${libPath}" $out/bin/${cratename}
                patchelf --add-rpath "${libPath}" $bin/bin/${cratename}
              '';
            };
          };
        mkOverride = cratename: overrideAttrs:
          p.rustBuilder.rustLib.makeOverride {
            name = cratename;
            inherit overrideAttrs;
          };
      in (if use-mold then
        [
          (p.rustBuilder.rustLib.makeOverride {
            overrideArgs = old: {
              rustcLinkFlags = (old.rustcLinkFlags or [ ]) ++ [
                "-C"
                "linker=${pkgs.clang}/bin/clang"
                "-C"
                "link-arg=-fuse-ld=${pkgs.mold}/bin/mold"
              ];
            };
          })
        ]
      else
        [ ]) ++ [
          (p.rustBuilder.rustLib.makeOverride {
            registry = "registry+https://github.com/rust-lang/crates.io-index";
            overrideArgs = old: {
              rustcLinkFlags = (old.rustcLinkFlags or [ ])
                ++ [ "--cap-lints" "warn" ];
            };
          })
        ] ++ (extra-overrides {
          inherit mkNativeDep mkEnvDep mkRpath mkOverride p;
        });
  };
in rustPkgs.workspace // { inherit (rustPkgs) workspaceShell; }
