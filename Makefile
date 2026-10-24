.PHONY: help build test lint fmt fmt-check check clean release integration-test

.DEFAULT_GOAL := help

help:
	@echo "Available targets:"
	@echo "  build             - compile"
	@echo "  test              - run unit + integration tests"
	@echo "  lint              - run clippy (-D warnings)"
	@echo "  fmt               - auto-format"
	@echo "  fmt-check         - check formatting (CI)"
	@echo "  integration-test  - run only integration tests"
	@echo "  release           - optimized release build"
	@echo "  clean             - remove build artifacts"
	@echo "  check             - lint + fmt-check + test"

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
