.PHONY: test
test:
	cargo fmt
	cargo test

.PHONY: build
build:
	cargo fmt
	cargo test
