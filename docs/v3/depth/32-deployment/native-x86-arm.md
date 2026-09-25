# 32-deployment/02 -- Native Deployment (x86_64 and aarch64)

> Build configuration, target triples, feature flags, cross-compilation,
> performance characteristics, and installation for native binary deployment.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `Cargo.toml` (workspace root), `.cargo/config.toml`

---

## 1. Supported Architectures

| Target Triple | OS | Arch | Linking | Primary Use |
|---------------|-----|------|---------|-------------|
| `x86_64-apple-darwin` | macOS | Intel | dynamic | Older Macs |
| `aarch64-apple-darwin` | macOS | Apple Silicon | dynamic | M1/M2/M3/M4 laptops |
| `x86_64-unknown-linux-gnu` | Linux | Intel | dynamic (glibc) | Servers, CI, Docker |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 | dynamic (glibc) | ARM servers (Graviton, Ampere) |
| `x86_64-unknown-linux-musl` | Linux | Intel | static (musl) | Docker slim images |
| `aarch64-unknown-linux-musl` | Linux | ARM64 | static (musl) | Docker slim, ARM containers |

The musl targets produce fully static binaries with no runtime libc
dependency. These are used for Docker slim images and for the prebuilt
binaries distributed via GitHub Releases.

The glibc targets are used for development builds and Docker full images
that need system packages (git, tmux, ttyd).

---

## 2. Toolchain Requirements

Roko requires Rust 1.91+ due to alloy dependencies (Ethereum primitives).
The green 2026-08-16 release checkpoint used rustc 1.96.1.

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
```

Running `rustup update stable` before building ensures the toolchain is
current.

---

## 3. Release Profile

```toml
[profile.release]
opt-level = 3
lto = "thin"        # Balance of compile time and binary size
codegen-units = 1   # Maximum optimization
strip = true        # Strip debug symbols
panic = "abort"     # Smaller binaries, no unwinding overhead
```

Development builds skip LTO and use multiple codegen units, keeping
incremental compile times at 10-30 seconds after the initial full build
(which takes 2-5 minutes for the workspace).

---

## 4. Feature Flags

Native builds enable all features by default. The feature system controls
which optional subsystems compile.

### roko-core Features

```toml
[features]
default = ["full"]
full = ["serde", "hdc", "decay"]
serde = ["dep:serde", "dep:serde_json"]
hdc = ["dep:roko-primitives"]
decay = []
```

### roko-cli Features

```toml
[features]
default = ["full"]
full = ["roko-agent/full", "roko-gate/full", "roko-compose/full", "tui"]
tui = ["dep:ratatui", "dep:crossterm"]
headless = []
```

### Default Native Build

```bash
cargo build --release -p roko-cli    # Full CLI with TUI, all backends
cargo build --release -p roko-serve  # HTTP API server
```

### Minimal Build (CI/headless server)

```bash
cargo build --release -p roko-cli --no-default-features --features headless
```

---

## 5. Cross-Compilation

### Using cargo-zigbuild (preferred, lighter)

```bash
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl -p roko-cli
cargo zigbuild --release --target aarch64-unknown-linux-musl -p roko-cli
```

cargo-zigbuild uses the Zig compiler as a drop-in C/C++ cross-compiler.
Faster than `cross` because no Docker required. cargo-dist uses this
approach for CI builds.

### Using cross (Docker-based)

```bash
cargo install cross
cross build --release --target x86_64-unknown-linux-musl -p roko-cli
cross build --release --target aarch64-unknown-linux-musl -p roko-cli
```

`cross` uses Docker containers with pre-configured toolchains per target.
Overhead is a Docker pull on first run (~500MB for the musl toolchain).

### Target-Specific Linker Config

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

---

## 6. Binary Size

Release binaries with all features, strip + LTO:

| Binary | Approximate Size | Notes |
|--------|-----------------|-------|
| `roko-cli` | ~25-35 MB | Full CLI with TUI, all gates, all backends |
| `roko-serve` | ~20-30 MB | HTTP API server, no TUI |

These sizes are typical for Rust binaries that include Tokio, Axum,
ratatui, and the alloy EVM stack. musl-linked binaries are slightly larger
due to static libc inclusion.

Size reduction strategies if needed:

- `opt-level = "z"` instead of `3` (~20% smaller, ~5% slower)
- Feature-gating: disable unused LLM backends for specific deployments
- Both `panic = "abort"` and `strip = true` are already configured

---

## 7. Performance Characteristics

Native deployment provides optimal performance:

1. **No abstraction overhead**: Binary runs directly on the host OS with
   no VM, container, or WASM interpreter overhead.
2. **Full SIMD**: HDC vector operations (Hamming distance, XOR bundling)
   use AVX2 on x86_64 and NEON on aarch64 via auto-vectorization.
3. **Full async runtime**: Tokio multi-threaded runtime uses all available
   cores. The adaptive clock in roko-runtime calibrates tick intervals
   based on system load.
4. **Direct filesystem access**: FileSubstrate uses direct JSONL I/O,
   memory-mapped files for the search index, and atomic file operations
   for crash-safe state.
5. **Full networking**: All 12 LLM provider kinds, MCP clients, WebSocket
   relay, HTTP serve endpoints.

### Memory Usage

| Scenario | RSS (approx.) |
|----------|--------------|
| roko-cli idle (after init) | ~50 MB |
| roko-cli running 4 agents | ~200-400 MB |
| roko-cli running 8 agents | ~400-800 MB |
| roko-serve idle | ~40 MB |

Memory scales primarily with concurrent agent count (each holds context
in memory) and code index size (tree-sitter ASTs + HDC fingerprints +
symbol graph).

---

## 8. Installation from Source

### Prerequisites

- Rust 1.91+ (install via `rustup`: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git (for cloning and for worktree operations)
- A C compiler (for tree-sitter FFI -- `cc` crate auto-detects)

### Build and Install

```bash
git clone https://github.com/nunchi/roko.git
cd roko
cargo build --workspace --release
cargo install --path crates/roko-cli
```

### Verify the Build

```bash
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo run -p roko-cli -- --version
cargo run -p roko-cli -- doctor
```

The `roko doctor` subcommand verifies the installation environment:
config files, API keys, gateway connectivity, index health, and git
availability.

---

## 9. Development Builds

For iterative development, debug builds compile much faster:

```bash
cargo build -p roko-cli                          # ~30-60s incremental
RUST_LOG=debug cargo run -p roko-cli -- status
cargo test -p roko-core -- signal
cargo test -p roko-gate -- compile_gate
```

Development builds skip LTO and use multiple codegen units, reducing
compile time from ~3-5 minutes (release) to ~30-60 seconds for
incremental rebuilds.

### Faster Linking

```bash
# mold linker (Linux) -- significantly faster link step
RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build -p roko-cli

# sccache for shared compilation cache
RUSTC_WRAPPER=sccache cargo build -p roko-cli
```

### Incremental Compilation

Rust's incremental compilation is enabled by default for debug builds.
After an initial full build (~2-5 minutes for the workspace), subsequent
builds that change a single crate typically complete in 10-30 seconds.

---

## 10. Implementation Status

> **Implementation status:** Native builds work on all six target triples.
> The workspace builds, tests (10,300+ tests), and passes clippy. The
> release checkpoint of 2026-08-16 used rustc 1.96.1. Cross-compilation
> has been validated in CI. Binary distribution via GitHub Releases is
> designed but not yet configured. The pre-commit checks (nightly fmt,
> clippy, tests) are mandatory for all commits.
