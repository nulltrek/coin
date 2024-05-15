.PHONY: test
test:
	cargo fmt
	cargo test -- --nocapture

.PHONY: build
build:
	cargo fmt
	cargo test
