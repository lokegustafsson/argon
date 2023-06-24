.PHONY: bench test

bench:
	nix run '.#bench'

test:
	nix run '.#test'
