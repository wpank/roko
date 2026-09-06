# Makefile — Developer and CI convenience targets for the roko workspace
#
# Prerequisites: rustup stable (>= 1.91), cargo, jq, bc
# Optional:      cargo-tarpaulin (for `coverage` target)
#
# Usage:
#   make test          — run the full workspace test suite
#   make clippy        — lint with -D warnings
#   make fmt           — format with nightly rustfmt
#   make check-metrics — validate headline efficiency metrics
#   make coverage      — measure test coverage via tarpaulin
#   make ci            — run all mandatory pre-commit checks

.PHONY: all test clippy fmt check-metrics coverage ci clean help

# Default: show help
all: help

# ── Rust toolchain targets ────────────────────────────────────────────────────

## Run the full workspace test suite
test:
	cargo test --workspace

## Run clippy with -D warnings (matches CI)
clippy:
	cargo clippy --workspace --no-deps -- -D warnings

## Format with nightly rustfmt (matches CI)
fmt:
	cargo +nightly fmt --all

## Check formatting without modifying files
fmt-check:
	cargo +nightly fmt --all -- --check

## Build the full workspace in debug mode
build:
	cargo build --workspace

## Build in release mode
build-release:
	cargo build --workspace --release

# ── CI and quality gates ──────────────────────────────────────────────────────

## Run all mandatory pre-commit checks (fmt-check, clippy, test)
ci: fmt-check clippy test
	@echo ""
	@echo "[ci] All pre-commit checks passed."

## Validate headline efficiency metrics from .roko/learn/efficiency.jsonl
check-metrics:
	@bash scripts/check-metrics.sh

## Measure test coverage for key crates via cargo-tarpaulin
coverage:
	@bash scripts/coverage.sh

# ── Workspace utilities ───────────────────────────────────────────────────────

## Remove build artifacts
clean:
	cargo clean

## Run workspace doctor
doctor:
	cargo run -p roko-cli -- doctor

# ── Help ──────────────────────────────────────────────────────────────────────

help:
	@echo "Roko workspace Makefile"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  test           Run the full workspace test suite"
	@echo "  clippy         Lint with cargo clippy -D warnings"
	@echo "  fmt            Format with nightly rustfmt (modifies files)"
	@echo "  fmt-check      Check formatting without modifying files"
	@echo "  build          Build workspace in debug mode"
	@echo "  build-release  Build workspace in release mode"
	@echo "  ci             Run all mandatory pre-commit checks"
	@echo "  check-metrics  Validate headline efficiency metrics"
	@echo "  coverage       Measure test coverage via cargo-tarpaulin"
	@echo "  clean          Remove build artifacts"
	@echo "  doctor         Run roko doctor"
	@echo "  help           Show this message"
