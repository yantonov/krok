.PHONY: build test lint fmt fmt-check check clean release integration-test

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

check: lint fmt-check test
	@echo "ALL CHECKS PASSED"

clean:
	cargo clean

release:
	cargo build --release

integration-test:
	cargo test --test cli
