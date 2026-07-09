# E05 — Gate Adaptivity On The Live Path

> Epic · backlog `tmp/status-quo/backlog/epics/` · HEAD `5852c93c05`
> Source docs: `101-TRACE-GATE-PIPELINE.md`, `35-GATES-VERIFICATION.md`, `25-PROOF-GATES.md`
> Depends on: **E01 — Runner Live Path** (`event_loop::run` is the single live executor; E05 edits its gate path).
> Related: **E02** (verdict sink/source split → `signals.jsonl` vs `engrams.jsonl`; VerdictPublisher / duplicate `GateVerdict` structs).

## Why this epic gates trustworthy autonomous execution

Roko self-executes only if a **green run means the work is actually verified**. Today it does not.
`roko plan run` → `event_loop::run` → `gate_dispatch::run_gate_once` builds the pipeline with
`RungExecutionInputs::default()` and never calls `enrich_rung_config`. Consequence: every advanced
rung (Symbol, GeneratedTest, VerifyChain, FactCheck, LlmJudge, Integration) returns
`Verdict::pass` unconditionally, those passes are folded into `all(verdicts.passed)`, and the single
per-rung EMA (only ever keyed at `max_gate_rung`=2) trends to 1.0 regardless of reality. The
adaptive-threshold, SPC/ratchet, oracle-4–6 and VerdictPublisher machinery the docs call "wired"
lives entirely on the **dead** `orchestrate.rs::PlanRunner`. An autonomous agent trusting these
verdicts is trusting a rubber stamp. E05 makes the live gate path honest: real inputs, neutral
(non-passing) stubs, per-rung stats that actually persist, and one rung dialect.

## Findings (verified this pass)

| # | Finding | Evidence (file:line) | Severity |
|---|---|---|---|
| F1 | Live gate path builds pipeline with `RungExecutionInputs::default()`; `enrich_rung_config`/`gate_rung_config` are only reachable from the dead `PlanRunner`. | `runner/gate_dispatch.rs:104-119`; `orchestrate.rs:18430,18492` | P0 |
| F2 | Advanced rungs stub-pass: `stub_verdict` returns `Verdict::pass`, aggregated by `all(passed)`. | `roko-gate/src/rung_dispatch.rs:290-296`, `:535`; `gate_dispatch.rs:140` | P0 |
| F3 | `GateCompletion.rung` is hard-labeled `pipeline_rung = ctx.config.max_gate_rung`; the executor's per-rung field is discarded → per-rung EMA only ever writes key `2`. | `event_loop.rs:4689`; `update_gate_thresholds` → `persist.rs:197` | P1 |
| F4 | Dead advance branch: `completion.passed && completion.rung < max_gate_rung` can never be true (`rung ≡ max_gate_rung`) → no incremental rung climb. | `event_loop.rs:1206` | P1 |
| F5 | `GateThresholds::save` (`pub(crate)`, writes `.roko/learn/gate-thresholds.json`) has **zero callers** in the live path; live threshold state hides only inside the executor snapshot. | `persist.rs:230-237`; snapshot at `event_loop.rs:3401,3415` | P1 |
| F6 | Three rung dialects: `rung_selector::CANONICAL_ORDER` (Symbol/GenTest/PropTest/Integration) vs `registry::GATE_SPECS` (diff/fmt/shell/judge) vs `effect_driver::rung_for_gate_name`; `confidence = if rung<=4 {1.0} else {0.5}` bakes dialect #2 numbering into affect policy. | `rung_selector.rs:96-128`; `registry.rs:134-184`; `effect_driver.rs:336,704` | P2 |
| F7 | `enable_advanced_rungs` dead toggle: both branches `skipped_count += 1`; also legacy-path-only. | `orchestrate.rs:18259-18270` | P2 |
| F8 | Escalation ladder inert: `select_rungs(complexity, caps, 0)` hard-codes `prior_failures = 0`. | `rung_dispatch.rs:123` | P3 |
| F9 | Verdicts written as ad-hoc JSON to `signals.jsonl`; dashboard reads `engrams.jsonl`; no VerdictPublisher / no real `Kind::GateVerdict` engram on live path. | `event_loop.rs:1147-1168`; `dashboard_snapshot.rs:1276` | P2 (shared w/ E02) |

## Reconciliation with P14 (`plans/P14-gate-rung-fix/tasks.toml`)

P14 predates the "two gate engines" finding. **All three of its tasks target the dead
`orchestrate.rs::PlanRunner`** (`selected_gate_steps`, the `enable_advanced_rungs` else-branch at
`orchestrate.rs:17965`) plus a roko-gate unit test. None touch `runner/event_loop.rs` or
`runner/gate_dispatch.rs`, so **P14 does not change any behaviour of `roko plan run`.**

| P14 task | What it does | Status vs E05 |
|---|---|---|
| P14-T1 | Push concrete gates in `orchestrate.rs::selected_gate_steps` when `enable_advanced_rungs` | **Superseded** — patches dead path (F7). Correct fix is E05-T05 (enrichment in live `gate_dispatch`). Recommend closing P14-T1 as "wrong engine". |
| P14-T2 | Upgrade that branch's log to info | **Superseded** with T1. |
| P14-T3 | roko-gate test: Complex pipeline has 7 gates | **Keep/absorb** — engine-agnostic unit test; still valid, folded into E05-T05 acceptance. |

E05 supersedes P14 as the live-path plan. P14 should be marked superseded (retain T3's test intent).

## Tasks (E05-Txx)

| ID | Title | Tier | Files | Depends | Gist |
|---|---|---|---|---|---|
| E05-T01 | Persist `GateThresholds` to the learn file on snapshot | focused | `runner/event_loop.rs`, `runner/persist.rs` | E01 | Call `GateThresholds::save(&paths.gate_thresholds_json)` wherever `save_snapshot` fires. (F5) |
| E05-T02 | Make stub verdicts neutral (`Skipped`), not `Verdict::pass` | focused | `roko-gate/src/rung_dispatch.rs` | E01 | `stub_verdict` → skipped verdict; `CanonicalRungGate` aggregation excludes skipped from `all(passed)`. (F2) |
| E05-T03 | Exclude skipped verdicts from EMA + `passed` in the runner | focused | `runner/gate_dispatch.rs`, `runner/event_loop.rs` | E05-T02 | `passed = verdicts.iter().filter(!skipped).all(passed)`; don't `observe()` on skipped-only rungs. (F2,F3) |
| E05-T04 | Label `GateCompletion` with the real inner rung; drive per-rung EMA | integrative | `runner/gate_dispatch.rs`, `runner/event_loop.rs`, `runner/types.rs` | E05-T03 | Emit per-selected-rung verdict/rung so `observe(rung,passed)` writes real keys, not just `2`; revive the advance branch or remove it deliberately. (F3,F4) |
| E05-T05 | Port `enrich_rung_config` → `RungExecutionInputs`/`RungExecutionConfig` into live `gate_dispatch` | architectural | `runner/gate_dispatch.rs`, `runner/event_loop.rs`, `roko-gate/src/rung_dispatch.rs` | E05-T04 | Attach SymbolManifest, source_roots, Perplexity `SearchOracle`, `AgentJudgeOracle`, integration pattern so advanced rungs actually verify. Absorbs P14-T3 test. (F1,F7) |
| E05-T06 | Unify the three rung dialects onto `rung_selector::Rung` | integrative | `roko-gate/src/registry.rs`, `roko-runtime/src/effect_driver.rs` | E05-T04 | One canonical rung numbering; delete `confidence = rung<=4` heuristic or map it through the canonical order. (F6) |
| E05-T07 | Remove the dead `enable_advanced_rungs` toggle (and close P14-T1/T2) | mechanical | `roko-cli/src/orchestrate.rs`, config | E05-T05 | Both branches skip; toggle is inert + legacy-only. Delete or wire to the live selector. (F7) |
| E05-T08 | Wire a VerdictPublisher on the live path (emit real `Kind::GateVerdict` engrams) | integrative | `runner/event_loop.rs` | E05-T04, E02 | Replace ad-hoc `signals.jsonl` JSON with a real published verdict the dashboard can read. Coordinate with E02. (F9) |

**Task count: 8** (E05-T01 … E05-T08). E05-T01/T02 are independent; the rest chain through T03→T04.

### First three tasks (native schema)

```toml
[meta]
plan = "E05-gate-adaptivity-live"
total = 3
done = 0
status = "ready"
max_parallel = 1

# ─────────────────────────────────────────────────────────────────────
# E05-T01 — Persist GateThresholds to the learn file on the live path
# ─────────────────────────────────────────────────────────────────────
# GateThresholds::save (persist.rs:230, pub(crate)) writes
# .roko/learn/gate-thresholds.json but has ZERO callers in the live
# runner. Live threshold state only rides inside the executor snapshot
# (event_loop.rs:3401), so `roko learn tune gates` and any cross-run
# adaptation read a file the runner never rewrites. Call save() wherever
# save_snapshot fires (and once at teardown).
[[task]]
id = "E05-T01"
title = "Persist GateThresholds to .roko/learn/gate-thresholds.json on snapshot"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 40
files = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/runner/persist.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runner/persist.rs", lines = "230-243", why = "GateThresholds::save + load_gate_thresholds; paths.gate_thresholds_json is the target" },
    { path = "crates/roko-cli/src/runner/persist.rs", lines = "40-80", why = "PersistPaths::gate_thresholds_json = layout.gate_thresholds_path()" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "3390-3420", why = "build_unified_snapshot serializes gate_thresholds into the executor snapshot only" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "900-1000", why = "save_snapshot call sites (:919,:995) where a threshold save should ride along" },
]
symbols = [
    "GateThresholds::save(&self, path: &Path) -> Result<()>",
    "PersistPaths { gate_thresholds_json: PathBuf, .. }",
]
anti_patterns = [
    "Do NOT change GateThresholds::observe or the EMA formula — this task only persists.",
    "Do NOT touch the legacy orchestrate.rs AdaptiveThresholds::save (5953) — different type, dead path.",
    "Do NOT make save() failures abort the run — log and continue (thresholds are best-effort telemetry).",
]

[[task.verify]]
phase = "structural"
command = "grep -nE 'gate_thresholds\\.save\\(|GateThresholds::save|\\.save\\(&?paths\\.gate_thresholds_json' crates/roko-cli/src/runner/event_loop.rs"
fail_msg = "GateThresholds::save must be called from the live runner (event_loop.rs)"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile"

acceptance = """
After a `roko plan run` over a Rust workspace, `.roko/learn/gate-thresholds.json` is
written/rewritten by the runner (mtime advances past process start, not only the executor
snapshot). save() is invoked on each save_snapshot and at teardown. No behavioural change
to gate pass/fail. `cargo check -p roko-cli` clean.
"""

# ─────────────────────────────────────────────────────────────────────
# E05-T02 — Make stub verdicts neutral (Skipped), not Verdict::pass
# ─────────────────────────────────────────────────────────────────────
# stub_verdict (rung_dispatch.rs:290-296) returns Verdict::pass for every
# unwired advanced rung (Symbol/GenTest/VerifyChain/FactCheck/LlmJudge/
# Integration). CanonicalRungGate aggregates with all(passed) (:535), so
# stubs silently make the aggregate PASS. A skipped/not-wired gate must be
# neutral, never a green checkmark. Use the skipped flag on Verdict (see
# roko_core::Verdict) or GateStatus::NotWired.
[[task]]
id = "E05-T02"
title = "stub_verdict returns a skipped (neutral) verdict, excluded from all(passed)"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 45
files = ["crates/roko-gate/src/rung_dispatch.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-gate/src/rung_dispatch.rs", lines = "285-310", why = "stub_verdict — currently Verdict::pass; make it skipped/not-wired" },
    { path = "crates/roko-gate/src/rung_dispatch.rs", lines = "520-545", why = "CanonicalRungGate aggregation with all(passed) — must exclude skipped" },
    { path = "crates/roko-gate/src/rung_dispatch.rs", lines = "245-296", why = "run_canonical_rung + which rungs feed stub_verdict" },
    { path = "crates/roko-core/src/foundation.rs", lines = "360-400", why = "Verdict/GateVerdict skipped/skip_reason fields to reuse" },
]
symbols = [
    "stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict",
    "Verdict { passed: bool, skipped: bool, .. }",
    "CanonicalRungGate::verify — aggregates inner verdicts",
]
anti_patterns = [
    "Do NOT delete the advanced rungs — they must appear as skipped/not-wired, not vanish.",
    "Do NOT flip stubs to Verdict::fail — an unwired gate failing would block every run; neutral is correct.",
    "Do NOT change Compile/Lint/Test/PropertyTest — those are real gates, untouched.",
]

[[task.verify]]
phase = "structural"
command = "grep -nE 'Verdict::pass' crates/roko-gate/src/rung_dispatch.rs | grep -iE 'stub' ; test $(grep -c 'skip' crates/roko-gate/src/rung_dispatch.rs) -ge 1"
fail_msg = "stub_verdict must produce a skipped verdict, not Verdict::pass"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-gate 2>&1"
fail_msg = "roko-gate tests must pass"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-gate 2>&1"
fail_msg = "roko-gate must compile"

acceptance = """
Unwired advanced rungs (Symbol/GeneratedTest/VerifyChain/FactCheck/LlmJudge/Integration)
produce verdicts with skipped=true (or GateStatus::NotWired) and a reason of "stub/not wired".
CanonicalRungGate's aggregate no longer counts a skipped inner verdict as a pass:
a pipeline whose only "successes" are stubs does NOT report passed=true. Existing real gates
unchanged. `cargo test -p roko-gate` green.
"""

# ─────────────────────────────────────────────────────────────────────
# E05-T03 — Exclude skipped verdicts from `passed` and from the EMA
# ─────────────────────────────────────────────────────────────────────
# gate_dispatch.rs:140 computes passed = verdicts.iter().all(|v| v.passed);
# once E05-T02 makes stubs skipped, they must be dropped from this fold
# (skipped != failed). event_loop.rs:1128 then calls
# update_gate_thresholds → observe(rung, passed): a rung whose verdicts are
# ALL skipped must NOT feed the EMA (it would inflate the pass rate toward
# 1.0 with no real signal — F2/F3).
[[task]]
id = "E05-T03"
title = "Skipped verdicts are neutral in passed-computation and never observed by the EMA"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 40
files = ["crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-cli/src/runner/event_loop.rs"]
role = "implementer"
depends_on = ["E05-T02"]

[task.context]
read_files = [
    { path = "crates/roko-cli/src/runner/gate_dispatch.rs", lines = "120-182", why = "passed = all(v.passed) and GateCompletion assembly — filter skipped" },
    { path = "crates/roko-cli/src/runner/event_loop.rs", lines = "1120-1170", why = "update_gate_thresholds / observe call site; skip when all inner verdicts are skipped" },
    { path = "crates/roko-cli/src/runner/persist.rs", lines = "195-228", why = "GateThresholds::observe — the EMA that must not see stub-only rungs" },
    { path = "crates/roko-cli/src/runner/types.rs", lines = "130-160", why = "GateVerdictSummary — carry a skipped flag through to completion" },
]
symbols = [
    "run_gate_once — passed = verdicts.iter().all(|v| v.passed)",
    "GateThresholds::observe(&mut self, rung: u32, passed: bool)",
    "GateVerdictSummary { gate_name, passed, .. }",
]
anti_patterns = [
    "Do NOT count a skipped verdict as failed — passed must ignore skipped, not fail on it.",
    "Do NOT observe() a rung when every non-verify verdict was skipped — that is the EMA-inflation bug.",
    "Do NOT drop skipped verdicts from the summaries/output — operators still need to see they were skipped.",
]

[[task.verify]]
phase = "structural"
command = "grep -nE 'skipped|!v\\.skipped|filter' crates/roko-cli/src/runner/gate_dispatch.rs"
fail_msg = "passed-computation in gate_dispatch must account for skipped verdicts"

[[task.verify]]
phase = "structural"
command = "grep -nE 'skipped|observe' crates/roko-cli/src/runner/event_loop.rs | grep -iE 'skip|observe'"
fail_msg = "EMA observe must be guarded against stub-only rungs"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile"

acceptance = """
`passed` in run_gate_once ignores skipped verdicts (they are neither pass nor fail). A rung
whose only non-verify verdicts are skipped stubs is NOT passed to GateThresholds::observe, so
`.roko/learn/gate-thresholds.json` (persisted by E05-T01) does not drift toward 1.0 from stub
passes. GateVerdictSummary carries the skipped flag through to GateCompletion so the TUI/ledger
can show "skipped". `cargo check -p roko-cli` clean.
"""
```

## Definition of done (epic)

- Live `roko plan run` produces `.roko/learn/gate-thresholds.json` with **more than one rung key**
  after runs across differing tiers (per-rung EMA is real, not a single scalar at rung 2).
- Stub/unwired gates report **skipped**, never `pass`; a run whose advanced rungs are all stubs
  cannot report a fully-green pipeline.
- Advanced rungs receive real `RungExecutionInputs` (Symbol/FactCheck/LlmJudge/Integration verify
  for real or are explicitly skipped — no silent pass).
- One rung dialect across `rung_selector`/`registry`/`effect_driver`; `confidence = rung<=4`
  heuristic removed or canonicalized.
- P14 marked superseded; its T3 test intent absorbed into E05-T05.
