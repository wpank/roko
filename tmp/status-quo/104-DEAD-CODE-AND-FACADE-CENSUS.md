# 104 — Dead Code & Feature-Façade Census

**Catalog of `#[cfg(feature=...)]` façades, orphan files (on disk, not in any `mod` tree),
and `#[allow(dead_code)]` hotspots.** Each row carries file:line and a keep/wire/delete verdict.

## Verification

- **HEAD**: `5852c93c05` on `main`
- **Date**: 2026-07-08
- **Scope**: `crates/**/src/**` only (tests excluded). Companion to
  **06-WIRING-STATUS.md** (which covers built-but-unwired *behavior*); this doc covers the
  *mechanical* dead-code surface: feature gates, orphans, and dead-code suppressions.
- **Status tags**: ✅ KEEP · 🔌 WIRE · 🗑️ DELETE · 🎭 FAÇADE (gate that gates nothing)

---

## Part 1 — Feature façades

`crates/roko-cli/Cargo.toml:13-20` declares three features. Their real effect on `src/`:

| Feature | Default? | `#[cfg]` guards in `src/` | Real effect | Tag |
|---|---|---|---|---|
| `legacy-orchestrate` | **OFF** (`Cargo.toml:16`) | **41** (`run.rs`:39, `lib.rs`:2) — plus the *entire* `orchestrate.rs` (~22K LOC) via `lib.rs:94-95` `#[cfg(feature="legacy-orchestrate")] pub mod orchestrate;` | Genuinely gates a massive legacy code path. Off by default → `orchestrate.rs` and its whole dependency web (conductor, MultiAgentPool, coordination-dreams) are **not compiled** in a default build | ✅ KEEP (real gate) but the gated code is legacy — see below |
| `legacy-runner-v2` | **ON** (`default = ["legacy-runner-v2"]`, `Cargo.toml:15`) | **0** — every `cfg(feature="legacy-runner-v2")` lives in `tests/` (`cost_dedup.rs:8`, `smoke.rs:199,311`, `phase0_wiring.rs:12`, `common/mod.rs:261`). **Zero in `src/`** | Toggling it changes **nothing** in the library build. The Cargo.toml comment claims it gates the runner-v2 plan path, but no `src/` code reads the flag. It only turns some integration tests on/off | 🎭 **FAÇADE** |
| (implicit no-feature = Graph engine) | — | — | The actual runtime selector is a **runtime CLI arg**, not a feature: `main.rs:1361 #[arg(long, default_value="graph")]`; dispatch at `commands/plan.rs:257` | ✅ (correct mechanism) |

### The façade in detail

`legacy-runner-v2` is a **default-on feature whose src footprint is empty**. A reader assumes
disabling it removes Runner-v2 from the binary; it does not — Runner-v2 (`runner/event_loop.rs`)
is always compiled and is reachable via `roko plan run --engine runner-v2` regardless of the flag.
The engine choice is a **runtime** `PlanEngine` enum (`main.rs:1301` default `RunnerV2`, but the
clap `default_value="graph"` at `main.rs:1361` overrides it, so the CLI default is the Graph engine).

**Verdict**: Remove `legacy-runner-v2` from `default` and either (a) add real `#[cfg]` guards so it
can actually exclude the runner-v2 source, or (b) delete the feature and keep the runtime `--engine`
switch as the single source of truth. Right now the feature is decorative.

### The legacy path it hides

`legacy-orchestrate` is a *real* gate, but what it gates is the single largest liability in the
tree: `crates/roko-cli/src/orchestrate.rs` (~22K LOC) is the **only** prod consumer of:
`roko-conductor` (all 10 watchers + breaker + diagnosis), `MultiAgentPool`, coordination-pattern
dreams, and live `conductor_load`. Because the feature is **off by default**, all of that is
dark in a normal build. See 06 for the per-symbol breakdown.

**Verdict**: Treat `orchestrate.rs` as scheduled-for-deletion. Anything worth keeping (conductor
supervision, live conductor_load) must be ported into `runner/event_loop.rs` before the feature
is removed, or it dies with it.

### Other feature gates (legitimate, keep)

| Feature | Where | Purpose | Tag |
|---|---|---|---|
| `alloy-backend` | `roko-chain` (`isfr_*.rs`, `lib.rs`) + consumed by `roko-cli/Cargo.toml:49` | Real blockchain backend gate (heavy alloy deps) | ✅ KEEP |
| feature gates in `roko-neuro` (`tier_progression.rs`, `knowledge_store.rs`, `context.rs`) | optional knowledge features | legitimate | ✅ KEEP |
| `roko-fs/src/file_substrate.rs`, `roko-primitives/src/hdc.rs`, `roko-lang-rust`, `roko-index` cfg gates | optional-dep gating | legitimate | ✅ KEEP |

---

## Part 2 — Orphan files (on disk, present, NOT in any `mod` tree → not compiled)

These `.rs` files exist under `src/` but no `mod`/`pub mod` declares them. Rust silently ignores
them. They are pure dead weight and, worse, other files carry `crate::` doc-links pointing at
their types (which are broken links resolving to the *runtime* twins).

| Orphan file | Duplicate of | Broken references to it | Tag |
|---|---|---|---|
| `crates/roko-core/src/state_hub.rs` (`StateHub` @71, `SharedStateHub`, `shared_state_hub`) | `roko-runtime::state_hub` (the compiled, wired one) | doc-links `roko-core/src/dashboard_snapshot.rs:5,755` | 🗑️ DELETE |
| `crates/roko-core/src/pulse_bus.rs` (`PulseBus` @29, wraps `EventBus<Pulse>`) | `roko-runtime::pulse_bus` (itself unwired) | `crate::PulseBus` doc-links `roko-core/src/bus_backends.rs:3`, `traits.rs:384` | 🗑️ DELETE |

Confirmed **no orphans** in `roko-runtime/src/` — all 24 non-`lib.rs` files are declared in
`lib.rs:43-67`. The two roko-core orphans are the only confirmed cases in the audited crates.

**Verdict**: delete both roko-core orphan files and fix the two `bus_backends.rs`/`traits.rs`
doc-links (they should point at `roko_runtime::pulse_bus::PulseBus` or be dropped).

---

## Part 3 — `#[allow(dead_code)]` hotspots

**71 occurrences across 37 files.** By crate:

| Crate | Count | Notable clusters |
|---|---|---|
| `roko-cli` | ~32 | tui (`app.rs`, `dashboard.rs`, `git_watch.rs`, `fs_watch.rs`, `jsonl_cursor.rs`, `rosedust.rs`), `doctor.rs:896-900`, `demo_seed.rs:125-153`, `auth.rs`, `orchestrate.rs:3542,20279,20285` |
| `roko-acp` | 10 | `bridge_events.rs:81-146` (8 fields) + `:4195` — the ACP bridge event structs carry many unused fields |
| `roko-agent` | 10 | `provider/openai_compat.rs:112-155` (6), `harness/acp_client.rs:250,253`, `cursor_cli_agent.rs:80,83`, `ollama/agent.rs:265` |
| `roko-daimon` | 4 | all in `phase2_stubs.rs:439,580,595,943` — phase2, expected |
| `roko-serve` | 4 | `terminal.rs:704` ("Retained for REST API extensibility"), `dispatch.rs:2492`, `routes/bench.rs:1335`, `status/helpers.rs:36` |
| `roko-std` | 3 | `scorer.rs:21,60`, `memory.rs:22` |
| `roko-orchestrator` | 2 | `safety/capability_tokens.rs:551,566` — part of the ~3.4K LOC dead safety dupe (see 06) |
| `roko-core`, `roko-agent-server`, `roko-chain`, `roko-dreams`, `roko-demo`, `roko-fs` | 1-2 each | mostly config/stub fields |

### Verdicts by category

| Category | Examples | Tag | Rationale |
|---|---|---|---|
| Phase-2 stubs | `roko-daimon/phase2_stubs.rs` (4), `roko-dreams/replay.rs:297` | ✅ KEEP (gated intent) | Intentional forward-decls; should be `#[cfg(feature="phase2")]` not `allow(dead_code)` |
| Provider option fields | `roko-agent/provider/openai_compat.rs:112-155` (6) | 🔌 WIRE | Deserialized config fields never read → silent config no-ops; wire into request building |
| ACP bridge fields | `roko-acp/bridge_events.rs:81-146` (8) | 🔌 WIRE | Ties to the unwired `request_permission` gate (see 06) — the event carries fields nothing consumes |
| Dead-safety-dupe | `roko-orchestrator/safety/capability_tokens.rs:551,566` | 🗑️ DELETE | Whole module is the 3.4K LOC dead duplicate |
| "Retained for future API" | `roko-serve/terminal.rs:704`, `dispatch.rs:2492` | ✅ KEEP-with-note | Honest suppressions; low risk |
| TUI/demo scaffolding | `roko-cli/tui/*`, `demo_seed.rs`, `doctor.rs` | ✅ KEEP | UI helper fields; churny by nature |
| Legacy orchestrate | `roko-cli/orchestrate.rs:3542,20279,20285` | 🗑️ DELETE-with-legacy | Dies with the legacy feature |

**Observation**: the two structurally-interesting clusters are (a) `openai_compat.rs` — six
config fields parsed from user config but never used → a config façade, and (b)
`bridge_events.rs` — the ACP permission event surface that has fields but no consumer, mirroring
the unwired `request_permission` gate.

---

## Roadmap

1. **Kill the façade**: drop `legacy-runner-v2` from `default`; either add real `src/` `#[cfg]`
   guards or delete the feature (keep runtime `--engine`). (`roko-cli/Cargo.toml:15`)
2. **Delete 2 orphan files** + fix their doc-links: `roko-core/src/{state_hub,pulse_bus}.rs`.
3. **Convert phase2 `allow(dead_code)` → `#[cfg(feature="phase2")]`**: daimon/dreams stubs, so
   the intent is enforced by the compiler, not a lint suppression.
4. **Wire or drop config façades**: `openai_compat.rs:112-155` (6 unread fields),
   `bridge_events.rs:81-146` (8 unread fields tied to the dead permission gate).
5. **Delete with the legacy feature**: `orchestrate.rs` dead-code fields once conductor
   supervision + live `conductor_load` are ported (or abandoned).

## Checklist

- [ ] Remove `legacy-runner-v2` from `default` (or give it real src guards)
- [ ] Delete `roko-core/src/state_hub.rs` + `roko-core/src/pulse_bus.rs`
- [ ] Fix broken `crate::PulseBus` doc-links (`bus_backends.rs:3`, `traits.rs:384`)
- [ ] Migrate phase2 `allow(dead_code)` to `cfg(feature="phase2")`
- [ ] Wire or delete the 6 `openai_compat` config fields
- [ ] Resolve the 8 `bridge_events` fields with the ACP permission gate decision
- [ ] Delete `roko-orchestrator/safety/` dead-code (tracked in 06)

See **06-WIRING-STATUS.md** for the behavioral built-but-unwired census these façades sit on top of.
