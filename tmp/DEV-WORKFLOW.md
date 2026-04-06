# Roko Dev Workflow

Fast iteration commands, tips, and recipes for working on roko day-to-day.

---

## Quick Reference

| What | Command |
|---|---|
| Watch + rebuild + serve | `roko-dev` |
| Demo frontend (HMR) | `roko-demo` |
| Both in one terminal | `roko-dev-full` |
| Release build (CLI only) | `roko-build` |
| Run release binary | `roko <subcommand>` |
| Docker dev (no rebuild) | `docker compose -f docker-compose.dev.yml up` |
| Docker restart after rebuild | `docker compose -f docker-compose.dev.yml restart roko` |

---

## Local Dev (fastest cycle)

### Terminal 1: Rust backend (auto-rebuild on save)

```bash
roko-dev
```

This runs `cargo watch -w crates/ -x "build -p roko-cli" -s "./target/debug/roko serve"`:
- Watches all files in `crates/`
- Rebuilds `roko-cli` on any `.rs` or `.toml` change
- Restarts `roko serve` on `:6677` after each successful build
- Typical rebuild time: 15-30s (incremental)

### Terminal 2: React frontend (instant HMR)

```bash
roko-demo
```

Opens Vite dev server at `http://localhost:5173`. All `/api/*` and `/ws/*` requests proxy to `:6677` automatically. React changes are instant via HMR.

### Combined (one terminal)

```bash
roko-dev-full
```

Runs both in parallel; Ctrl+C kills both.

---

## Docker Dev (testing the container without full rebuild)

The Docker image takes 10-15 min to build from scratch because it compiles Rust inside the container. For iteration, bind-mount your locally-built binaries instead:

```bash
# One-time: build release binaries locally
cargo build --release -p roko-cli -p mirage-rs -p agent-relay

# Start container with local binaries mounted in
docker compose -f docker-compose.dev.yml up

# After changing Rust code:
cargo build --release -p roko-cli
docker compose -f docker-compose.dev.yml restart roko
```

This skips the entire Docker build stage. The container uses your local binaries directly.

### Full Docker rebuild (for CI/deploy testing)

```bash
docker build -t roko:dev .
docker run --rm -p 6677:6677 \
  -e ZHIPU_API_KEY \
  roko:dev
```

---

## Zed Integration

`~/bin/roko-dev` is a symlink to `target/debug/roko`. After any `cargo build -p roko-cli`, Zed's agent sessions can use the latest binary without copying or restarting Zed — just restart the agent session.

---

## Building

### Debug (fast compile, slow runtime)

```bash
cargo build -p roko-cli              # just the CLI
cargo build --workspace              # everything
```

### Release (slow compile, fast runtime)

```bash
roko-build                           # alias for release CLI build
cargo build --release --workspace    # everything in release
```

### Check only (fastest — no linking)

```bash
cargo check -p roko-cli              # just CLI
cargo check --workspace              # everything
```

Useful when you only care if it compiles, not about running it.

---

## Testing

### cargo-nextest (2-3x faster, parallel, better output)

```bash
roko-test                                        # alias: nextest all
cargo nextest run --workspace                    # same thing
cargo nextest run -p roko-agent                  # single crate
cargo nextest run --workspace -E 'test(snapshot)' # filter by name
cargo nextest run --workspace --no-capture       # see println output
```

### Standard cargo test (when nextest isn't needed)

```bash
cargo test --workspace               # all tests
cargo test -p roko-agent             # single crate
cargo test -p roko-cli -- snapshot   # grep test name
cargo test --workspace -- --nocapture  # see println output
```

### Pre-commit (mandatory before pushing)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## Useful cargo-watch Recipes

```bash
# Check on save (fastest feedback — no codegen/linking)
cargo watch -w crates/ -x 'check -p roko-cli'

# Test on save
cargo watch -w crates/ -x 'test -p roko-agent'

# Clippy on save
cargo watch -w crates/ -x 'clippy -p roko-cli --no-deps -- -D warnings'

# Build + run a specific command
cargo watch -w crates/ -x 'build -p roko-cli' -s './target/debug/roko status'
```

---

## Demo App (frontend)

```bash
cd demo/demo-app

npm run dev          # Dev server with HMR at :5173
npm run build        # Production build to dist/
npm run e2e          # Playwright end-to-end tests
npm run e2e:headed   # E2E with browser visible
```

The Vite config ignores `.roko/`, `target/`, and `roko.toml` changes so roko-serve restarts don't trigger full page reloads.

---

## Performance Tips

### Keep swap low

The #1 cause of sluggish terminal/Claude is swap pressure. Monitor with:

```bash
sysctl vm.swapusage
```

If swap > 20GB, close unused:
- Claude/Codex sessions (`ps aux | grep claude`)
- Cursor/Zed windows (each spawns MCP servers)
- Browser tabs (Chrome renderers add up fast)

### Cargo build cache

Incremental cache (`target/debug/incremental/`) grows unbounded. Nuke it periodically:

```bash
rm -rf target/debug/incremental/   # frees 10-30GB, next build is slower once
```

Or full clean:

```bash
cargo clean --profile dev           # removes all debug artifacts
```

### Git

This repo has 700+ commits and 25GB in pack files. Keep git fast with:

```bash
git config core.fsmonitor true      # already set
git config core.untrackedCache true # already set
```

Periodically repack (run when idle — uses lots of RAM):

```bash
git repack -ad --threads=4
```

### sccache (compilation cache)

Configured in `.cargo/config.toml`. Caches compiled crate artifacts — huge win when:
- Switching branches (deps don't recompile)
- After `cargo clean` (deps hit cache)
- CI (shared cache across runs)

```bash
sccache --show-stats              # see hit rate
sccache --zero-stats              # reset counters
SCCACHE_CACHE_SIZE=10G            # set in ~/.zshrc
```

Cache lives at `~/Library/Caches/Mozilla.sccache/`.

### Spotlight exclusion

`target/` and `.git/` have `.metadata_never_index` files to prevent Spotlight from indexing build artifacts (reduces random disk I/O during builds).

### Shell startup

Already optimized:
- NVM lazy-loaded (saves 600ms)
- compinit cached (saves 660ms)
- brew --prefix hardcoded (saves 90ms)
- Starship `$rust` module disabled (saves 20ms/prompt)

### Worktrees

Stale worktrees slow every git operation. List and prune:

```bash
git worktree list                   # see all
git worktree remove <path> --force  # remove merged ones
git worktree prune                  # clean stale refs
```

---

## Aliases Summary (in ~/.zshrc)

```bash
# Run release binary
alias roko='/Users/will/dev/nunchi/roko/roko/target/release/roko'
alias roko-build='cargo build --release -p roko-cli --manifest-path /Users/will/dev/nunchi/roko/roko/Cargo.toml'

# Dev workflow (auto-rebuild)
alias roko-dev='cd /Users/will/dev/nunchi/roko/roko && cargo watch -w crates/ -x "build -p roko-cli" -s "./target/debug/roko serve"'
alias roko-demo='cd /Users/will/dev/nunchi/roko/roko/demo/demo-app && npm run dev'
alias roko-dev-full='cd /Users/will/dev/nunchi/roko/roko && (trap "kill 0" EXIT; cargo watch -w crates/ -x "build -p roko-cli" -s "./target/debug/roko serve" & cd demo/demo-app && npm run dev)'

# Mirage
alias mirage='/Users/will/dev/nunchi/roko/roko/target/release/mirage-rs'
alias mirage-build='cargo build --release -p mirage-rs --manifest-path /Users/will/dev/nunchi/roko/roko/Cargo.toml'
```

---

## Common Workflows

### "I changed a route in roko-serve and want to test it"

```bash
# If roko-dev is running: just save the file. It rebuilds + restarts automatically.
# If not:
cargo build -p roko-cli && ./target/debug/roko serve
# Then hit http://localhost:6677/api/your-route
```

### "I changed the demo frontend"

```bash
# If roko-demo is running: just save. HMR applies instantly.
# If not:
cd demo/demo-app && npm run dev
```

### "I want to test in Docker like production"

```bash
cargo build --release -p roko-cli -p mirage-rs -p agent-relay
docker compose -f docker-compose.dev.yml up
# Edit Rust, rebuild, restart:
cargo build --release -p roko-cli && docker compose -f docker-compose.dev.yml restart roko
```

### "I want to run a single roko command to test"

```bash
cargo build -p roko-cli && ./target/debug/roko status
cargo build -p roko-cli && ./target/debug/roko run "hello world"
cargo build -p roko-cli && ./target/debug/roko prd list
```

### "I want to profile what's slow"

```bash
# Time a build
cargo build -p roko-cli --timings    # opens HTML report

# Check what's linking
cargo build -p roko-cli -v 2>&1 | grep "Linking"
```

### "Kill everything roko-related"

```bash
pkill -f "roko serve"
pkill -f "cargo watch"
pkill -f "mirage-rs"
# Demo app:
pkill -f "vite"
```

---

## Installed Build Tooling

| Tool | What it does | How it's used |
|---|---|---|
| `cargo-watch` | File watcher, rebuilds on save | `roko-dev`, `roko-check` aliases |
| `sccache` | Compilation cache (10GB) | Auto via `.cargo/config.toml` |
| `cargo-nextest` | Parallel test runner (2-3x faster) | `roko-test` alias |
| `mold` | Fast linker (ELF only — Linux/Docker) | Used in Docker builds |
| `lld` | LLVM linker | Available but not default on macOS |

### What's NOT worth doing on macOS arm64:

- **mold linker**: Only supports ELF (Linux). macOS uses Mach-O. Apple's linker is already fast for arm64.
- **Cranelift backend**: `rustc` nightly-only, unstable, no real win for this project size.
- **RAM disk for target/**: With SSD and 64GB RAM, the OS page cache already does this.
- **Parallel frontends (`-Z threads=8`)**: Nightly-only. Marginal gains.

### What DOES help on macOS arm64:

- **sccache**: Avoids recompiling unchanged deps. Biggest win after branch switches.
- **`opt-level = 1` in dev**: Already configured. Compiles faster than `-O0` (less LLVM work on large generics).
- **`opt-level = 2` for deps**: Already configured. Deps compile once, run fast.
- **`split-debuginfo = "unpacked"`**: Already configured. Faster linking (no DWARF bundling).
- **`cargo check` instead of `cargo build`**: 2-3x faster when you don't need to run the binary.
- **`cargo nextest`**: Runs tests in parallel processes (not threads). 2-3x faster for large test suites.
- **Spotlight exclusion**: `.metadata_never_index` in `target/` and `.git/`.
- **git fsmonitor + untrackedCache**: Faster `git status` (starship calls this every prompt).
