.PHONY: run-integration-tests

run-integration-tests:
	cargo test --test "*" -- --nocapture