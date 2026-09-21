# Default recipe: list available commands
default:
    @just --list

# Build the CLI binary
build:
    cargo build -p roko-cli

# Run all tests
test:
    cargo test --workspace

# Format code (nightly, matches CI)
fmt:
    cargo +nightly fmt --all

# Lint (must pass clean before commit)
clippy:
    cargo clippy --workspace --no-deps -- -D warnings

# Pre-commit checks: fmt, clippy, test (in that order)
check: fmt clippy test

# Run roko CLI via cargo (rebuilds if needed)
run *ARGS:
    cargo run -p roko-cli -- {{ARGS}}

# Run roko with pre-built debug binary
roko *ARGS:
    target/debug/roko {{ARGS}}

# Run a plan directory with the pre-built binary
plan-run DIR:
    target/debug/roko plan run plans/{{DIR}}

# Speed test with Cerebras provider
speed-test:
    rm -rf .roko/state/graph/speed-cerebras && target/debug/roko plan run plans/speed-cerebras

# Show provider health
providers:
    target/debug/roko config providers health

# Run doctor diagnostics
doctor:
    target/debug/roko doctor

# Clean build artifacts
clean:
    cargo clean
