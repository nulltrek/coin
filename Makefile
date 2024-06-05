.PHONY: test-print
test-print:
	cargo fmt
	cargo test -- --nocapture

.PHONY: test
test:
	cargo fmt
	cargo test

.PHONY: doc
doc:
	cargo doc --document-private-items

.PHONY: build
build:
	cargo fmt
	cargo build
