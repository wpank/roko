# doctor-network-v2 — Implementation Plan

## What this plan does

Implements `roko doctor network` — a focused subcommand that probes the HTTP
reachability and round-trip latency of every configured LLM provider endpoint,
running all probes concurrently and reporting structured results.

## Why it's needed

The existing `roko doctor` validates config, layout, keys, and local serve health
but never verifies that the host can actually reach remote provider APIs. Silent
network failures surface as 30–120s timeouts in agent dispatch. This command lets
operators identify network-layer problems before any task is dispatched.

## Key design decisions (from sibling PRD `doctor-network-probe.md`)

- **Opt-in, not always-on.** `roko doctor` (no subject) gains zero new HTTP calls.
  Network probing is triggered only via `roko doctor network`.
- **Parallel fan-out.** All provider probes run concurrently via `tokio::task::JoinSet`.
  Wall-clock time = slowest single probe, not the sum.
- **Reuses existing infrastructure.** `reqwest` is already in `doctor.rs` (line 7),
  `Duration` is already imported, and `run_disk_doctor` establishes the exact dispatch
  pattern to mirror.
- **Status semantics.** 2xx = Ok; 401/403 = Warn ("endpoint reachable, auth needed");
  5xx/conn error = Fail; CLI-only or network-deny = Skipped.
- **Zero new dependencies.** `reqwest`, `tokio`, and `serde` are all already used by
  `roko-cli`.

## File impact summary

| File | Change |
|------|--------|
| `crates/roko-cli/src/main.rs` | +1 variant to `DoctorSubject` |
| `crates/roko-cli/src/doctor.rs` | +~150 LOC across T2/T3/T4/T6 |
| `crates/roko-cli/src/commands/util.rs` | +~15 LOC wiring the Network arm |
| `crates/roko-cli/tests/doctor.rs` | +~30 LOC integration test |

## Dependency chain
