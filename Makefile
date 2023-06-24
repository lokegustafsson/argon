.PHONY: bench compare test flamegraph

bench:
	nix run '.#bench'

compare:
	nix run '.#compare'

test:
	nix run '.#test'

flamegraph:
	nix run '.#flamegraph'
