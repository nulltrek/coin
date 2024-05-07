.PHONY: test
test:
	cargo test

.PHONY: build
build:
	cargo fmt
	cargo test
