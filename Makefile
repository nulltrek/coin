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
	cargo doc

.PHONY: run-server
run-server:
	cargo run --bin server
