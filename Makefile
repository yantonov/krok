.PHONY: help help-all build test lint fmt fmt-check check clean release integration-test

.DEFAULT_GOAL := help

help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "  check    - Lint + fmt-check + test"
	@echo "  build    - Compile (debug)"
	@echo "  release  - Optimized release build"
	@echo "  test     - Run unit + integration tests"
	@echo ""
	@echo "Run 'make help-all' for the full list, including internal targets."

help-all:
	@echo "Public targets:"
	@echo "  check    - Lint + fmt-check + test"
	@echo "  build    - Compile (debug)"
	@echo "  release  - Optimized release build"
	@echo "  test     - Run unit + integration tests"
	@echo ""
	@echo "Internal targets (typically called via check):"
	@echo "  lint             - Run clippy (-D warnings)"
	@echo "  fmt              - Auto-format"
	@echo "  fmt-check        - Check formatting (CI)"
	@echo "  integration-test - Run only integration tests"
	@echo "  clean            - Remove build artifacts"

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
