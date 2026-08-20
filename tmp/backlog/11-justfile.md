# 11 — Justfile: Developer Convenience Command Runner

**Priority**: P3 — nice-to-have DX improvement; not blocking any core functionality
**Size**: XS (half day)
**Crates**: None — this is a plain text file at the workspace root
**Depends on**: None

---

## Background

Roko is a Rust workspace with 35 crates at `/Users/will/dev/nunchi/roko/roko/`. When working on it, developers need to run several cargo commands with specific flags that differ from the simpler defaults. For example, formatting requires `cargo +nightly fmt --all` (not the default `cargo fmt`), and clippy requires `cargo clippy --workspace --no-deps -- -D warnings`. These exact incantations are documented in `CLAUDE.md` but have no single entry point.

`just` is a command runner (similar conceptually to `make`, but with simpler syntax and no build system semantics) available at https://github.com/casey/just. It reads a `justfile` at the project root and exposes named recipes. Running `just ci` is easier than remembering 3 different cargo commands with non-obvious flags.

The predecessor system (`bardo`) had a `justfile` at the repo root. This item recreates that convenience layer for roko. The key insight is that this adds zero runtime overhead — `just` is purely a shortcut layer over commands that already work independently.

There is currently no `justfile` at `/Users/will/dev/nunchi/roko/roko/`. Verified: running `ls /Users/will/dev/nunchi/roko/roko/` shows no justfile present.

## Current State

1. No `justfile` exists at the workspace root `/Users/will/dev/nunchi/roko/roko/`.
2. The pre-commit requirements are documented in `/Users/will/dev/nunchi/roko/roko/CLAUDE.md` under "Pre-commit checks (MANDATORY before any commit)":
   - `cargo +nightly fmt --all`
   - `cargo clippy --workspace --no-deps -- -D warnings`
   - `cargo test --workspace`
3. The main binary crate is `roko-cli` at `crates/roko-cli/`. Run via `cargo run -p roko-cli -- <subcommand>`.
4. The workspace Cargo.toml is at `/Users/will/dev/nunchi/roko/roko/Cargo.toml`.

## Implementation Plan

### Step 1: Install `just` (prerequisite for the developer, not for CI)

```bash
cargo install just
# or: brew install just
```

### Step 2: Create the justfile

Create `/Users/will/dev/nunchi/roko/roko/justfile` with this exact content:

```just
# Default: list available recipes
default:
    @just --list

# ── Build ──────────────────────────────────────────────────────────────────

# Build all workspace members (debug)
build:
    cargo build --workspace

# Build release binary
build-release:
    cargo build --workspace --release

# Type-check without codegen (fast feedback)
check:
    cargo check --workspace

# ── Test ───────────────────────────────────────────────────────────────────

# Run all workspace tests
test:
    cargo test --workspace

# Run tests for a specific crate: `just test-crate roko-learn`
test-crate crate:
    cargo test -p {{crate}}

# Generate HTML coverage report (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --workspace --html

# ── Lint & Format ──────────────────────────────────────────────────────────

# Format with nightly rustfmt (matches CI requirement)
fmt:
    cargo +nightly fmt --all

# Check formatting without modifying files (used in CI)
fmt-check:
    cargo +nightly fmt --all -- --check

# Run clippy with workspace-wide deny-warnings (matches CI requirement)
lint:
    cargo clippy --workspace --no-deps -- -D warnings

# Run cargo-deny license/advisory checks (requires cargo-deny)
deny:
    cargo deny check

# ── CI ─────────────────────────────────────────────────────────────────────

# Full CI gate: fmt-check + lint + test (mirrors pre-commit requirements from CLAUDE.md)
ci: fmt-check lint test

# ── Documentation ──────────────────────────────────────────────────────────

# Build workspace docs
doc:
    cargo doc --workspace --no-deps

# Build and open workspace docs in browser
doc-open:
    cargo doc --workspace --no-deps --open

# ── Runtime ────────────────────────────────────────────────────────────────

# Start HTTP control plane on :6677
serve:
    cargo run -p roko-cli -- serve

# Start interactive ratatui dashboard
dashboard:
    cargo run -p roko-cli -- dashboard

# Run any roko CLI command: `just run status`, `just run plan list`
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

### Step 3: Verify it works

```bash
cd /Users/will/dev/nunchi/roko/roko
just --list        # should print all recipe names
just check         # should run cargo check --workspace
just ci            # should run fmt-check, lint, test in sequence
just run status    # should run roko status
```

### Step 4: Add the justfile to git tracking

The file is plain text and should be committed to the repository. It is not sensitive and adds no CI overhead.

## Acceptance Criteria

1. `just --list` from `/Users/will/dev/nunchi/roko/roko/` prints all recipe names without error.
2. `just ci` runs `cargo +nightly fmt --all -- --check`, then `cargo clippy --workspace --no-deps -- -D warnings`, then `cargo test --workspace`, in sequence, and exits non-zero if any step fails.
3. `just serve` starts `roko serve` equivalently to running `cargo run -p roko-cli -- serve`.
4. `just dashboard` starts the ratatui TUI equivalently to `cargo run -p roko-cli -- dashboard`.
5. `just fmt` applies nightly formatting and the result matches what `cargo +nightly fmt --all` would produce directly.
6. `just run status` executes `cargo run -p roko-cli -- status` and exits with the same code.

## Verification Checklist

- [ ] Run `just --list` from the workspace root — should print a formatted recipe table
- [ ] Run `just check` — should complete without error (same as `cargo check --workspace`)
- [ ] Run `just fmt-check` — should report any formatting issues (or exit 0 if already formatted)
- [ ] Run `just lint` — should pass clean with no warnings
- [ ] Run `just test` — should run all 9,900+ tests and pass
- [ ] Run `just ci` — should run all three CI steps sequentially; verify it exits non-zero if you introduce a deliberate formatting error
- [ ] Run `just run status` — should output roko status information
- [ ] Run `just serve` — should start the HTTP control plane on :6677 (Ctrl-C to stop)
- [ ] Run `git status` to confirm `justfile` appears as an untracked file ready to add

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/justfile` | Create this new file with the content above |
