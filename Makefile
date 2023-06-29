.PHONY: bench compare test flamegraph

bench:
	nix run '.#bench'

compare:
	nix run '.#compare'

test:
	nix run '.#test'

flamegraph:
	nix run '.#flamegraph'

cargo2nix-extra:
	nix run '.#cargo2nix-extra'
