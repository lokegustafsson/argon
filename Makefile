.PHONY: \
	bench \
	cargo2nix-extra \
	compare \
	flamegraph \
	test

bench:
	nix run '.#bench'

cargo2nix-extra:
	nix run '.#cargo2nix-extra'

compare:
	nix run '.#compare'

flamegraph:
	nix run '.#flamegraph'

test:
	nix run '.#test'
