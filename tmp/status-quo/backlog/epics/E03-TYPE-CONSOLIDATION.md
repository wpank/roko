# E03 — Type Consolidation

> **Epic header**
> - Epic ID: `E03`
> - Repo HEAD: `5852c93c05a4f1bda8ff880fc752d9fba2ba453e` (branch `main`)
> - Source docs: `103-DUPLICATE-TYPES-CENSUS`, `47-FOUNDATION-TYPES-REDESIGN`, `76-DATA-CONTRACTS-SCHEMAS`
> - Prior plan coverage: **NONE** — this is a pure gap. No `plans/*` task touches type consolidation.
> - Blast radius: **HIGH** — 19 cross-crate duplicated type families, ~zero conversions between them.
> - Unblocks: **E02 (Storage)** — depends on a single `RetentionPolicy`; **E10 (Frontend/Dashboard)** — depends on a single `DashboardSnapshot` fed by StateHub.

## Problem statement

The census (`103`) found 19 type families with the same name defined at 2–4 different
sites, carrying a **cross-crate runtime contract** (state that flows between plan execution,
gates, dashboard, storage, learning, chain). **Conversion coverage is almost nil**: the only
`impl From<…>` that exist are one-directional adapters *into* learning/gate outcome types
(`From<&GateVerdict> for ProviderModelGateOutcome` at `roko-learn/src/provider_model_outcome.rs:65`,
`From<&GateVerdict> for GateStatus` at `roko-gate/src/registry.rs:56`). **Zero** `From`/`Into`
exist *between* competing definitions of the same-named family. Every crossing is a manual
re-map or a hard wall — the structural root of dashboard emptiness and nondeterministic
retention.

This epic canonicalizes the highest-blast families and either (a) collapses duplicates into
the canonical owner, or (b) disambiguates genuinely-different views by rename + adds
`From`/`Into` adapters so state can flow losslessly.

## Per-type-family table

| Family | Definition sites (verified) | Canonical owner | Difficulty | Strategy |
|---|---|---|---|---|
| **StateHub (orphan)** | `roko-core/src/state_hub.rs` (NOT declared in `lib.rs` — dead file) · live one: `roko-runtime/src/state_hub.rs:80` | `roko-runtime::state_hub` | Low (mechanical) | **Delete** core orphan file |
| **GateVerdict** ×4 | `roko-core/src/foundation.rs:368` (canonical, exec) · `roko-core/src/dashboard_snapshot.rs:290` (ring view) · `roko-learn/src/episode_logger.rs:90` (hashed record) · `roko-chain/src/identity_economy_identity.rs:1600` (stub) | `roko-core::foundation` | High | Canonical + `From` adapters, then rename divergent copies |
| **DashboardSnapshot** ×3 | `roko-core/src/dashboard_snapshot.rs:759` (rich, StateHub-fed) · `roko-cli/src/runner/projection.rs:124` (thin, events-only) · `roko-cli/src/tui/dashboard.rs:3308` (file-scraper) | `roko-core::dashboard_snapshot` | High | Rename thin/TUI copies; TUI consumes core snapshot via `watch::Receiver` |
| **RetentionPolicy** ×3 | `roko-fs/src/gc.rs:32` (whole `.roko/` GC) · `roko-serve/src/retention.rs:20` (per-artifact rotation) · `roko-learn/src/episode_logger.rs:1229` (episode compaction) | *new shared* (`roko-core` or `roko-fs`) | Med | Introduce shared policy + adapters into the 3 engines |
| **Engram (dead dup)** | `roko-core/src/engram.rs:63` (real) · `roko-chain/src/identity_economy_markets.rs:653` (forensic-replay stub, never wired) | `roko-core::engram` | Low (mechanical) | **Delete** chain stub |
| **Cell (trait)** ×2 | `roko-core/src/cell.rs:91` (verb-trait supertrait) · `roko-graph/src/cell.rs:74` (graph-node) | `roko-core::cell` | High | Out of scope for E03 core pass — tracked, deferred (needs adapter, two kernels) |
| **DispatchPlan** (semantic) ×3 | `roko-core/src/dispatch_plan.rs:75` · `ExecutorAction::DispatchPlan` variant · cli `RunnerDispatchPlan` | keep separate | Low | Rename for grep-clarity (optional tail task) |

Full 19-family register lives in `103-DUPLICATE-TYPES-CENSUS.md §1`. E03 attacks the five
runtime-critical families above (rows with a concrete owner + strategy); the semantic-collide
renames (rows 9, 14, 16, 17) and the two-kernel `Cell` split are logged as follow-ups.

## Ordering (by blast radius + de-risking)

Mechanical orphan/dead deletions first (shrink the surface, zero contract risk), then
GateVerdict as the **template** that proves the canonical + `From`-adapter pattern, then the
highest-blast **DashboardSnapshot** unification (which reuses that pattern), then the
storage-critical **RetentionPolicy** share, then the remaining dead-dup cleanup.

1. `E03-T01` Delete orphan `roko-core::state_hub` — **mechanical**
2. `E03-T02` GateVerdict: establish canonical + add `From` adapters — **standard**
3. `E03-T03` GateVerdict: rename divergent copies (struct count → 1) — **standard**
4. `E03-T04` DashboardSnapshot: rename thin/TUI copies to disambiguated names — **mechanical**
5. `E03-T05` DashboardSnapshot: TUI consumes `roko-core::DashboardSnapshot` via `watch::Receiver` — **complex**
6. `E03-T06` RetentionPolicy: introduce shared canonical + adapters into 3 engines — **standard**
7. `E03-T07` Delete dead `roko-chain::Engram` stub — **mechanical**

## Tasks

### E03-T01 — Delete orphan `roko-core::state_hub`
- **tier**: mechanical
- **files**: `crates/roko-core/src/state_hub.rs` (delete)
- **depends_on**: none
- **acceptance**: file removed; no `mod state_hub` reference remains in roko-core; `roko-core` still compiles (file was never declared as a module, so this is dead weight).
- **verify**: `test ! -f crates/roko-core/src/state_hub.rs` · `! rg -q 'mod state_hub' crates/roko-core/src/` · `cargo check -p roko-core`

### E03-T02 — GateVerdict: canonical + From adapters
- **tier**: standard
- **files**: `crates/roko-core/src/foundation.rs`, `crates/roko-core/src/dashboard_snapshot.rs`, `crates/roko-learn/src/episode_logger.rs`
- **depends_on**: none
- **acceptance**: `foundation::GateVerdict` documented as the single canonical gate result; `impl From<&foundation::GateVerdict>` exists producing the dashboard-ring view and the episode record (lossy fields documented). Gate producers keep emitting the foundation shape; downstream views derive via `From`.
- **verify**: conversion-exists checks + `cargo check -p roko-core -p roko-learn`

### E03-T03 — GateVerdict: rename divergent copies
- **tier**: standard
- **files**: `crates/roko-core/src/dashboard_snapshot.rs`, `crates/roko-learn/src/episode_logger.rs`, `crates/roko-chain/src/identity_economy_identity.rs`
- **depends_on**: `E03-T02`
- **acceptance**: only `foundation::GateVerdict` retains the bare name; the ring view → `GateVerdictView`, episode record → `GateVerdictRecord`, chain stub → `ChainGateVerdict`. All call sites updated. `From` adapters from T02 retargeted to renamed types.
- **verify**: `[ "$(rg -c 'struct GateVerdict \{' crates/ | wc -l)" -eq 1 ]` (exactly one file defines bare `struct GateVerdict {`) · `cargo check --workspace`

### E03-T04 — DashboardSnapshot: rename thin/TUI copies
- **tier**: mechanical
- **files**: `crates/roko-cli/src/runner/projection.rs`, `crates/roko-cli/src/tui/dashboard.rs`
- **depends_on**: none
- **acceptance**: projection copy → `ProjectionSnapshot`; TUI copy → `TuiDashboardModel`. Only `roko-core::dashboard_snapshot::DashboardSnapshot` keeps the bare name. Pure rename, no behaviour change yet.
- **verify**: `[ "$(rg -c 'struct DashboardSnapshot \{' crates/ | wc -l)" -eq 1 ]` · `cargo check -p roko-cli`

### E03-T05 — DashboardSnapshot: TUI consumes core snapshot
- **tier**: complex
- **files**: `crates/roko-cli/src/tui/dashboard.rs`, `crates/roko-cli/src/tui/` (render path)
- **depends_on**: `E03-T04`
- **acceptance**: TUI receives `roko_core::DashboardSnapshot` via a `watch::Receiver` (same StateHub feed `roko-serve` reads) instead of scraping `.roko/learn/*.json` + `.roko/engrams.jsonl`. `TuiDashboardModel` becomes a `From<&roko_core::DashboardSnapshot>` render projection. File-scraping fallback kept only when no StateHub sender is attached (offline mode).
- **verify**: `rg -q 'watch::Receiver.*DashboardSnapshot|roko_core::.*DashboardSnapshot' crates/roko-cli/src/tui/` · `cargo check -p roko-cli` · manual: `roko dashboard` renders non-empty when `roko serve`/StateHub is publishing.

### E03-T06 — RetentionPolicy: shared canonical + adapters
- **tier**: standard
- **files**: `crates/roko-fs/src/gc.rs`, `crates/roko-serve/src/retention.rs`, `crates/roko-learn/src/episode_logger.rs`, (new) shared type in `roko-core` or `roko-fs`
- **depends_on**: none
- **acceptance**: one shared `RetentionPolicy` (superset: `max_episodes`, `max_age_days`, `max_run_age_days`, `max_archive_age_days`, `size_threshold_mb`, per-artifact `strategy`). The 3 engines consume it (directly or via `From`), so a single config governs episode retention. No engine prunes `episodes.jsonl` under a rule the others can't see.
- **verify**: conversion-exists / shared-import checks (`rg -q 'RetentionPolicy' crates/roko-fs crates/roko-serve crates/roko-learn` all referencing the shared type) · `cargo check -p roko-fs -p roko-serve -p roko-learn`

### E03-T07 — Delete dead `roko-chain::Engram` stub
- **tier**: mechanical
- **files**: `crates/roko-chain/src/identity_economy_markets.rs` (remove `struct Engram` + refs)
- **depends_on**: none
- **acceptance**: the never-wired forensic-replay `Engram` stub is removed; `roko-core::engram::Engram` is the only `Engram`.
- **verify**: `[ "$(rg -c 'struct Engram \{' crates/ | wc -l)" -eq 1 ]` · `cargo check -p roko-chain`

## First 3 tasks — executable TOML

```toml
[meta]
plan = "E03-type-consolidation"
total = 7
done = 0
status = "ready"
max_parallel = 1

# ── E03-T01: Delete orphan roko-core::state_hub ──
#
# crates/roko-core/src/state_hub.rs is a byte-near clone of the live
# roko-runtime::state_hub, but it is NOT declared as a module in
# roko-core/src/lib.rs (no `mod state_hub;`) and no external crate imports
# roko_core::state_hub. It is dead weight that will silently drift from the
# runtime copy. Everyone uses roko-runtime's (re-exported by roko-serve and
# roko-cli). Pure deletion — nothing compiles it today.

[[task]]
id = "E03-T01"
title = "Delete orphan roko-core::state_hub dead file"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 5
files = ["crates/roko-core/src/state_hub.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-core/src/state_hub.rs", lines = "1-90", why = "The orphan file to delete — confirm it is StateHub/SharedStateHub/StateHubSender only" },
    { path = "crates/roko-core/src/lib.rs", lines = "1-120", why = "Confirm there is NO `mod state_hub;` declaration — the file is not even compiled" },
]
symbols = [
    "struct StateHub — roko-core/src/state_hub.rs (orphan; live one is roko-runtime/src/state_hub.rs:80)",
]
anti_patterns = [
    "Do NOT touch roko-runtime/src/state_hub.rs — that is the live one everyone uses.",
    "Do NOT add a re-export from core to runtime — that inverts the dependency graph. Delete the core copy.",
    "Do NOT delete dashboard_snapshot.rs doc-comment references to StateHub — they point at the runtime type conceptually.",
]

[[task.verify]]
phase = "structural"
command = "test ! -f crates/roko-core/src/state_hub.rs"
fail_msg = "orphan state_hub.rs must be deleted"

[[task.verify]]
phase = "structural"
command = "! rg -q 'mod state_hub' crates/roko-core/src/"
fail_msg = "no module declaration for state_hub may remain in roko-core"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core 2>&1"
fail_msg = "roko-core must compile cleanly after deletion"


# ── E03-T02: GateVerdict canonical + From adapters ──
#
# foundation::GateVerdict (gate_name, passed, skipped, skip_reason, output,
# duration_ms) is the shape a gate actually produces and the one
# GateReport { verdicts: Vec<GateVerdict> } binds to. Make it THE canonical
# gate result and add From adapters that project it into the dashboard-ring
# view (dashboard_snapshot.rs:290) and the hashed episode record
# (episode_logger.rs:90). Do NOT rename yet (that is E03-T03) — this task only
# establishes the canonical + the lossy conversions so downstream views stop
# hand-re-mapping. Two From<&GateVerdict> adapters already exist into learning
# outcome types; follow that pattern.

[[task]]
id = "E03-T02"
title = "Establish foundation::GateVerdict as canonical + add From adapters into dashboard/episode views"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 90
files = [
    "crates/roko-core/src/foundation.rs",
    "crates/roko-core/src/dashboard_snapshot.rs",
    "crates/roko-learn/src/episode_logger.rs",
]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-core/src/foundation.rs", lines = "360-400", why = "Canonical GateVerdict {gate_name,passed,skipped,skip_reason,output,duration_ms}" },
    { path = "crates/roko-core/src/dashboard_snapshot.rs", lines = "285-310", why = "Ring view GateVerdict {plan_id,task_id,gate,passed,ts_millis} — target of a From adapter" },
    { path = "crates/roko-learn/src/episode_logger.rs", lines = "85-110", why = "Episode record GateVerdict {gate,passed,signature} — hashed, target of a From adapter" },
    { path = "crates/roko-gate/src/registry.rs", lines = "50-70", why = "Existing From<&GateVerdict> for GateStatus — the adapter pattern to mirror" },
    { path = "crates/roko-learn/src/provider_model_outcome.rs", lines = "60-80", why = "Existing From<&GateVerdict> for ProviderModelGateOutcome — pattern reference" },
]
symbols = [
    "struct GateVerdict — roko-core/src/foundation.rs:368 (canonical)",
    "struct GateVerdict — roko-core/src/dashboard_snapshot.rs:290 (ring view)",
    "struct GateVerdict — roko-learn/src/episode_logger.rs:90 (episode record)",
]
anti_patterns = [
    "Do NOT change the field set of foundation::GateVerdict — it is the source of truth.",
    "Do NOT delete the dashboard/episode structs in this task — renaming/collapsing is E03-T03.",
    "Do NOT invent plan_id/task_id inside the From impl — those must be threaded in by the caller; if the ring view needs context the From should take a (verdict, context) tuple or a small builder, not fabricate ids.",
    "Preserve the hashing of the episode `signature` field — never store raw gate output there.",
]

[[task.verify]]
phase = "structural"
command = "rg -q 'impl From<.*foundation::GateVerdict|impl From<&GateVerdict>' crates/roko-core/src/dashboard_snapshot.rs crates/roko-learn/src/episode_logger.rs"
fail_msg = "From adapters from canonical GateVerdict into dashboard/episode views must exist"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-core -p roko-learn 2>&1"
fail_msg = "roko-core and roko-learn must compile with the new adapters"


# ── E03-T03: Rename divergent GateVerdict copies ──
#
# After E03-T02 the conversions exist. Now disambiguate names so exactly ONE
# `struct GateVerdict {` remains (the foundation canonical). Ring view ->
# GateVerdictView, episode record -> GateVerdictRecord, chain stub ->
# ChainGateVerdict. Update all call sites + retarget the T02 From impls to the
# renamed types. This kills grep noise and makes the canonical unambiguous.

[[task]]
id = "E03-T03"
title = "Rename divergent GateVerdict copies so exactly one bare struct GateVerdict remains"
status = "ready"
tier = "standard"
model_hint = "claude-sonnet-4-5"
max_loc = 120
files = [
    "crates/roko-core/src/dashboard_snapshot.rs",
    "crates/roko-learn/src/episode_logger.rs",
    "crates/roko-chain/src/identity_economy_identity.rs",
]
role = "implementer"
depends_on = ["E03-T02"]

[task.context]
read_files = [
    { path = "crates/roko-core/src/dashboard_snapshot.rs", lines = "285-310", why = "Rename ring-view GateVerdict -> GateVerdictView + update all references in this file" },
    { path = "crates/roko-learn/src/episode_logger.rs", lines = "85-110", why = "Rename episode-record GateVerdict -> GateVerdictRecord + update references" },
    { path = "crates/roko-chain/src/identity_economy_identity.rs", lines = "1595-1620", why = "Rename chain stub GateVerdict -> ChainGateVerdict" },
]
symbols = [
    "struct GateVerdict — foundation.rs:368 (KEEP this name — canonical)",
    "struct GateVerdict — dashboard_snapshot.rs:290 (RENAME -> GateVerdictView)",
    "struct GateVerdict — episode_logger.rs:90 (RENAME -> GateVerdictRecord)",
    "struct GateVerdict — identity_economy_identity.rs:1600 (RENAME -> ChainGateVerdict)",
]
anti_patterns = [
    "Do NOT rename foundation::GateVerdict — it must remain the single bare `struct GateVerdict`.",
    "Do NOT break the From adapters added in E03-T02 — update their target types to the renamed structs.",
    "Do NOT leave dangling `use ...::GateVerdict` imports that now resolve to the wrong type; update each import to the renamed symbol or the foundation canonical, whichever the call site means.",
    "roko-core/src/forensic.rs:124 GateVerdictRecord already exists — pick a non-colliding name for the episode rename (e.g. EpisodeGateVerdict) if GateVerdictRecord collides after cross-crate import.",
]

[[task.verify]]
phase = "structural"
command = "[ \"$(rg -c 'struct GateVerdict \\{' crates/ | wc -l)\" -eq 1 ]"
fail_msg = "exactly one file must define bare `struct GateVerdict {` after rename (the foundation canonical)"

[[task.verify]]
phase = "compile"
command = "cargo check --workspace 2>&1"
fail_msg = "workspace must compile after GateVerdict rename + call-site updates"
```

## Downstream unblocks

- **E02 (Storage)** blocks on `E03-T06` — a single shared `RetentionPolicy` is prerequisite
  for deterministic `.roko/` retention; storage work must not build on three competing GC rules.
- **E10 (Frontend/Dashboard)** blocks on `E03-T04`/`E03-T05` — the dashboard-emptiness root is
  the DashboardSnapshot split-brain; the TUI must consume the StateHub-fed
  `roko-core::DashboardSnapshot` before any frontend polish is meaningful.

## Follow-ups (out of E03 core scope, logged)

- `Cell` two-kernel split (`roko-core::cell` vs `roko-graph::cell`) — needs an adapter, High.
- Semantic-collide renames: `DispatchPlan`×3, `TaskState`×2, `Verdict`×2, `Outcome`×2, `Plan`×3 —
  rename-for-clarity, Low each; batch as a tail task once the runtime-critical families land.
- `AgentState`×4, `TaskStatus`×3, `GateFeedback`×3, `EventBus`×4 — additional SPLIT-BRAIN
  families from `103 §1` deferred to a follow-on consolidation epic.
