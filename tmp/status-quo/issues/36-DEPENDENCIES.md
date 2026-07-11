# Workspace Dependency Issues

## Critical — Layering violations

### roko-runtime (layer 1) depends on roko-gate (layer 3)
- `roko-runtime/Cargo.toml`: `roko-gate = { path = "../roko-gate" }` in `[dependencies]`.
- Low-level async runtime primitive depending on high-level gate logic.

### roko-std (layer 2) depends on roko-chain (layer 2)
- `roko-std/Cargo.toml:15`: `roko-chain = { path = "../roko-chain" }`.
- Every crate using `roko-std` (5 crates) gets full alloy/EVM in transitive deps.
- Should be an optional feature.

## High — Duplicate versions

82 duplicate crate+version pairs. Significant:

| Crate | Versions | Cause |
|-------|----------|-------|
| `reqwest` | 0.12 + 0.13 | `alloy` (roko-chain) pulls 0.13 |
| `rand` | 0.8 + 0.9 | `tokenizers` (roko-compose) pulls 0.9 |
| `tokio-tungstenite` | 0.26 + 0.28 | Two callers pin different versions |
| `notify` | 6.x + 8.x | Workspace pins 6, `roko-plugin` hardcodes 8.2.0 |
| `thiserror` | 1.x + 2.x | alloy ecosystem |
| `jsonwebtoken` | 9.x + 10.x | `octocrab` requires 10.x |
| `getrandom` | 0.2 + 0.3 + 0.4 | Three semver series |
| `hashbrown` | 0.14 + 0.15 + 0.16 + 0.17 | Multiple transitive pins |

## Medium — Tightly coupled same-layer cluster

`roko-agent`, `roko-learn`, `roko-neuro`, `roko-compose`, `roko-dreams` form a bidirectional dependency cluster at layer 2. Cycles avoided only by dev-dependency placement. Design intent of separate crates undercut.

## Low — Heavy optionalizable deps

- `tokenizers` in `roko-compose`: HuggingFace lib pulls `rand 0.9`, `fancy-regex`, `compact_str`. Should be feature-gated.
- `roko-std → roko-chain`: EVM stack for all consumers. Should be optional.
