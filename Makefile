.PHONY: test-print
test-print:
	cargo fmt
	cargo test -- --nocapture

.PHONY: test
test:
	cargo fmt
	cargo test
