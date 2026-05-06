# ACP Batch Manifest

18 batches in 4 groups. Creates the `roko-acp` crate implementing ACP JSON-RPC server over stdio.

## Dependency graph

```
scaffold:
  ACP01 ──► ACP02 ──► ACP03

core:
  ACP03 ──► ACP04 ──► ACP05 ──► ACP06 ──► ACP07 ──► ACP08
                         │         │
config:                  │         │
  ACP05 ──► ACP15       │         │
  ACP05 ──► ACP16       │         │
  ACP05 ──► ACP17       │         │
                         │         │
bridges:                 │         │
  ACP06 ──► ACP09       │         │
  ACP06 ──► ACP10       │         │
  ACP06 ──► ACP11       │         │
  ACP06 ──► ACP12       │         │
  ACP06 ──► ACP13       │         │
  ACP06 ──► ACP14       │         │
                         │         │
final:                   │         │
  ACP07 + ACP15 + ACP16 ──► ACP18
```

## Serial execution order

```
ACP01 ACP02 ACP03 ACP04 ACP05 ACP06 ACP07 ACP08
ACP09 ACP10 ACP11 ACP12 ACP13 ACP14
ACP15 ACP16 ACP17 ACP18
```

## Batch manifest

| Batch | Title | Group | Deps | Verify | ~LOC |
|-------|-------|-------|------|--------|------|
| ACP01 | Scaffold `roko-acp` crate + workspace wire | scaffold | — | cargo check | ~300 |
| ACP02 | ACP JSON-RPC types (inline, no SDK dep) | scaffold | ACP01 | check + clippy | ~600 |
| ACP03 | Stdio transport layer | scaffold | ACP02 | test + clippy | ~400 |
| ACP04 | Handler dispatch loop | core | ACP03 | check + clippy | ~400 |
| ACP05 | Session management | core | ACP04 | test + clippy | ~500 |
| ACP06 | Prompt handling + event streaming | core | ACP05 | check + clippy | ~600 |
| ACP07 | `roko acp` CLI subcommand | core | ACP06 | check (roko-cli) + clippy | ~250 |
| ACP08 | Protocol conformance tests | core | ACP07 | cargo test | ~400 |
| ACP09 | File system bridge | bridges | ACP06 | check + clippy | ~300 |
| ACP10 | Terminal bridge | bridges | ACP06 | check + clippy | ~350 |
| ACP11 | Permission bridge | bridges | ACP06 | check + clippy | ~250 |
| ACP12 | Gate result bridge | bridges | ACP06 | check + clippy | ~300 |
| ACP13 | Plan phase bridge | bridges | ACP06 | check + clippy | ~250 |
| ACP14 | Usage/cost bridge | bridges | ACP06 | check + clippy | ~200 |
| ACP15 | Session config options | config | ACP05 | test + clippy | ~400 |
| ACP16 | Slash commands | config | ACP05 | check + clippy | ~300 |
| ACP17 | Elicitation forms | config | ACP05 | check + clippy | ~300 |
| ACP18 | Lifecycle integration tests | config | ACP07 ACP15 ACP16 | cargo test | ~500 |

**Estimated total**: ~6,300 LOC

## Execution cadence

### Night 1: scaffold + core (ACP01–ACP08)
```bash
bash tmp/acp-runner/run-acp.sh --group scaffold,core
```
8 batches, serial. Creates the crate, all types, transport, handler, sessions, streaming, CLI command, and conformance tests.

**After Night 1**: `roko acp` compiles and handles basic protocol lifecycle.

### Night 2: bridges (ACP09–ACP14)
```bash
bash tmp/acp-runner/run-acp.sh --continue last --group bridges
```
6 batches, all independent (only depend on ACP06). FS, terminal, permissions, gates, plans, usage.

**After Night 2**: Editor-mediated I/O works.

### Night 3: config + tests (ACP15–ACP18)
```bash
bash tmp/acp-runner/run-acp.sh --continue last --group config
```
4 batches. Config options, slash commands, elicitation, integration tests.

**After Night 3**: Full Phase 1–3 ACP functionality.

## Verify gates (all batches)

1. **Scope** — Only `crates/roko-acp/` modified (+ `crates/roko-cli/` for ACP07, + root `Cargo.toml` for ACP01)
2. **Diff** — Produced some changes
3. **Required terms** — Expected vocabulary present
4. **Cargo check** — Compiles (temp CARGO_TARGET_DIR, cleaned after)
5. **Clippy** — Clean (`-D warnings`)
6. **Tests** — Batch-specific (ACP03, ACP05, ACP08, ACP15, ACP18)

## Commit format

```
acp(ACP01): Scaffold roko-acp crate + workspace wire
```

## Scope rules

| Batch | Allowed paths |
|-------|--------------|
| ACP01 | `crates/roko-acp/`, `Cargo.toml` |
| ACP07 | `crates/roko-acp/`, `crates/roko-cli/` |
| All others | `crates/roko-acp/` |
