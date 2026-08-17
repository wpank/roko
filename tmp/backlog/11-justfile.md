# Backlog: Justfile — Developer Convenience Commands

**Status**: Backlog
**Priority**: P3 (nice to have)
**Size**: XS (half day)
**Origin**: `tmp/architecture-archive/21-tui-and-operations.md` (Section 4: Justfile)

---

## Problem Statement

The roko workspace has no top-level developer convenience wrapper. Common operations require typing long `cargo` invocations or consulting `CLAUDE.md` to recall the exact flags:

- `cargo +nightly fmt --all` (not `cargo fmt`)
- `cargo clippy --workspace --no-deps -- -D warnings` (not `cargo clippy`)
- `cargo test --workspace`
- `cargo run -p roko-cli -- serve`
- `cargo run -p roko-cli -- dashboard`
- `cargo llvm-cov --workspace --html` for coverage

The predecessor system (`bardo/justfile`, 136 lines) had a `just` task file at the repo root that encoded all of these. New contributors and automated tooling (CI scripts, editor integrations) benefit from a single canonical entry point.

`just` is a widely adopted command runner (https://github.com/casey/just) installable via `cargo install just` or Homebrew. Its syntax is Makefile-inspired but simpler, and it handles argument passing, environment defaults, and recipe dependencies cleanly. This is not a build system — it is a shortcut layer over commands that already work.

---

## Proposed Solution

Create a `justfile` at the workspace root (`/Users/will/dev/nunchi/roko/roko/justfile`) with the following recipes:

```just
# Default: list available recipes
default:
    @just --list

# ── Build ──────────────────────────────────────────────────────────────────

# Build all workspace members
build:
    cargo build --workspace

# Build release binary
build-release:
    cargo build --workspace --release

# Type-check without codegen
check:
    cargo check --workspace

# ── Test ───────────────────────────────────────────────────────────────────

# Run all workspace tests
test:
    cargo test --workspace

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}}

# Generate HTML coverage report (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --workspace --html

# ── Lint & Format ──────────────────────────────────────────────────────────

# Format with nightly rustfmt (matches CI)
fmt:
    cargo +nightly fmt --all

# Check formatting without modifying files
fmt-check:
    cargo +nightly fmt --all -- --check

# Run clippy with workspace-wide deny-warnings
lint:
    cargo clippy --workspace --no-deps -- -D warnings

# Run cargo-deny license/advisory checks (requires cargo-deny)
deny:
    cargo deny check

# ── CI ─────────────────────────────────────────────────────────────────────

# Full CI gate: fmt-check + lint + test (mirrors pre-commit requirements)
ci: fmt-check lint test

# ── Documentation ──────────────────────────────────────────────────────────

# Build workspace docs
doc:
    cargo doc --workspace --no-deps

# Build and open docs in browser
doc-open:
    cargo doc --workspace --no-deps --open

# ── Runtime ────────────────────────────────────────────────────────────────

# Start HTTP control plane on :6677
serve:
    cargo run -p roko-cli -- serve

# Start interactive ratatui dashboard
dashboard:
    cargo run -p roko-cli -- dashboard

# Run a single roko CLI command: `just run status`
run *args:
    cargo run -p roko-cli -- {{args}}

# ── Dev Utilities ──────────────────────────────────────────────────────────

# Watch: re-check on file change (requires cargo-watch)
watch:
    cargo watch -x 'check --workspace'

# Remove build artifacts
clean:
    cargo clean
```

The recipe list is intentionally conservative: it wraps commands that already exist and are documented in `CLAUDE.md`. It does not introduce new logic.

---

## Implementation Location

| File | Path |
|---|---|
| Justfile | `/Users/will/dev/nunchi/roko/roko/justfile` (repo root, tracked in git) |

No Rust code changes. No Cargo.toml changes. The file is plain text and adds no CI overhead.

Optional follow-up: add `just ci` to the GitHub Actions CI matrix as a sanity check that the justfile recipes are not broken.

---

## Acceptance Criteria

1. `just --list` from the repo root prints all recipe names without error.

2. `just ci` runs `cargo +nightly fmt --all -- --check`, `cargo clippy --workspace --no-deps -- -D warnings`, and `cargo test --workspace` in sequence; it exits non-zero if any step fails.

3. `just serve` starts `roko serve` (equivalent to `cargo run -p roko-cli -- serve`).

4. `just dashboard` starts the ratatui TUI.

5. `just fmt` applies nightly formatting and the working tree diff matches what `cargo +nightly fmt --all` would produce.

---

## References

- Source spec: `/Users/will/dev/nunchi/roko/roko/tmp/architecture-archive/21-tui-and-operations.md` (Section 4)
- Predecessor reference: `bardo/justfile` (136 LOC, not in this repo)
- Pre-commit requirements: `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` (Pre-commit checks section)
- `just` tool: https://github.com/casey/just
