{ pkgs, large-file-json, argonBin, cargo2nix, patched-simd-json-src }: {
  bench = {
    runtimeInputs = [ argonBin pkgs.bash pkgs.coreutils pkgs.gron pkgs.wget ];
    text = ''
      cd ''${TMPDIR:-/tmp}
      printf "@ $(pwd)\n"
      set -v
      time argon ${large-file-json} > large-file.argon
      time argon --ungron large-file.argon > large-file.json
    '';
  };
  compare = {
    runtimeInputs = [ argonBin pkgs.bash pkgs.coreutils pkgs.gron pkgs.wget ];
    text = ''
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
    '';
  };
  test = {
    runtimeInputs = [
      argonBin
      pkgs.bash
      pkgs.coreutils
      pkgs.diffutils
      pkgs.gron
      pkgs.wget
    ];
    text = ''
      printf "\nComparing on escaping.json.."
      G1=$(sha256sum <(gron ${./testcases/escaping.json}))
      A1=$(sha256sum <(argon ${./testcases/escaping.json}))
      diff <(echo $G1) <(echo $A1)

      printf "\nComparing on large-file.json.."
      G2=$(sha256sum <(gron ${large-file-json}))
      A2=$(sha256sum <(argon ${large-file-json}))
      diff <(echo $G2) <(echo $A2)
    '';
  };
  flamegraph = {
    runtimeInputs =
      [ argonBin pkgs.bash pkgs.coreutils pkgs.cargo-flamegraph ];
    text = ''
      flamegraph -- argon ${large-file-json} > /dev/null
    '';
  };
  cargo2nix-extra = {
    runtimeInputs = [
      cargo2nix
      pkgs.bash
      pkgs.coreutils
    ];
    text = ''
      BASE=$(basename "$(pwd)")
      FLAKE="$(pwd)/flake.nix"
      if [[ ("$BASE" != "argon") || (! -f "$FLAKE") ]]; then
          echo "Must be run from the argon source tree root!"
          exit 1
      fi
      set -v
      rm ./crates/patched-simd-json || true
      ln -s ${patched-simd-json-src} ./crates/patched-simd-json
      cargo2nix -f
    '';
  };
}
