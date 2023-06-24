.PHONY: build bench

TMP=/tmp/argon-test-folder

$(TMP)/large-file.json:
	mkdir $(TMP)
	cd $(TMP) && wget 'https://raw.githubusercontent.com/json-iterator/test-data/master/large-file.json'

build:
	cargo build --release

bench: build $(TMP)/large-file.json
	time gron $(TMP)/large-file.json > $(TMP)/gron.result
	time ./target/release/argon $(TMP)/large-file.json > $(TMP)/argon.result
