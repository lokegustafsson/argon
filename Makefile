.PHONY: bench compare test

bench:
	nix run '.#bench'

compare:
	nix run '.#compare'

test:
	nix run '.#test'
