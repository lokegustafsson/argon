{ pkgs, large-file-json, rust, cargo2nix, patched-simd-json-src }: {
  bench = {
    runtimeInputs = [ pkgs.bash pkgs.coreutils pkgs.gron pkgs.wget rust.argon ];
    text = ''
      cd ''${TMPDIR:-/tmp}
      printf "@ $(pwd)\n"
      set -v
      time argon ${large-file-json} > large-file.argon
      time argon --ungron large-file.argon > large-file.json
    '';
  };
  compare = {
    runtimeInputs = [ pkgs.bash pkgs.coreutils pkgs.gron pkgs.wget rust.argon ];
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
      pkgs.bash
      pkgs.coreutils
      pkgs.diffutils
      pkgs.gron
      pkgs.wget
      rust.argon
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
      [ pkgs.bash pkgs.coreutils pkgs.cargo-flamegraph rust.argon ];
    text = ''
      flamegraph -- argon ${large-file-json} > /dev/null
    '';
  };
  cargo2nix-extra = {
    runtimeInputs = [
      pkgs.bash
      pkgs.coreutils
      cargo2nix
    ];
    text = ''
      BASE=$(basename "$(pwd)")
      if [ "$BASE" != "argon" ]; then
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
