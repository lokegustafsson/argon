.PHONY: \
	bench \
	cargo2nix-extra \
	compare \
	flamegraph \
	integration-test \
	unit-test

bench:
	nix run '.#bench'

cargo2nix-extra:
	nix run '.#cargo2nix-extra'

compare:
	nix run '.#compare'

flamegraph:
	nix run '.#flamegraph'

integration-test:
	nix run '.#integration-test'

unit-test:
	nix build '.#unit-test' -L
