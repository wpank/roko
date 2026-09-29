+++
id = "spec-1ced1d"
kind = "spec"
title = "Cross-crate change-impact scoping before dispatch: public-contract impact fixtures and the index-based consumer oracle"
status = "open"
triage = "verified"
severity = "p2"
size = "L"
subsystem = ["roko-cli/impact-analysis"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/backlog/231-cross-crate-change-impact-scoping.md"
anchors = ["crates/roko-cli/src/runner/impact_analysis.rs::analyze", "crates/roko-cli/src/runner/preflight.rs::check_declared_change_impact", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/dispatch/prompt_builder.rs::declared_impact_context", "crates/roko-cli/src/runner/cargo_command.rs::focused_verify_steps", "crates/roko-cli/src/commands/impact.rs::cmd_impact"]
goal = "core"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_impact_gate_compiles_reverse_dependents' crates/roko-cli/ && cargo test -p roko-cli graph_impact_gate_compiles_reverse_dependents && ! grep -A1 -E '\"gates\\.impact_(timeout_ms|max_reverse_dependents|max_targets)\"' crates/roko-cli/src/graph_task_dispatch.rs | grep -q LEGACY_GATES"
+++

## Problem

When a task changes a public contract (a struct field `bool` -> `Option<bool>`, a trait, a re-exported
enum, a serde type), the consumers in other crates must change too and must compile. The original
dogfood failure: the task listed five files, missed more than ten call sites in `roko-serve` and
`roko-cli`, failed two compile gates, and was repaired by hand.

Backlog #231 built a conservative answer in August (`d43dd45cd`, `38f79a5ca`): diff + `cargo metadata`
analysis, a pre-dispatch scope warning, an impact section in the agent prompt, and a post-diff gate that
compiles bounded reverse dependents. Most of that runtime wiring lived in Runner-v2. Since Runner-v2 was
deleted (2026-09-06), a Graph `roko plan run` of such a task:

- gets **no pre-dispatch warning**: `runner/preflight.rs::run_preflight_checks` (which runs
  `check_declared_change_impact`) has no caller;
- gets **no reverse-dependent compile**: `impact_analysis::analyze` and `focused_verify_steps` are only
  called from `runner/gate_dispatch.rs::run_gate_once`, which the Graph path does not use. The
  `gates.impact_timeout_ms`, `gates.impact_max_reverse_dependents` and `gates.impact_max_targets` keys
  are reported as inert by `graph_engine_inert_settings`;
- still gets the prompt's impact policy, which tells the agent "The runner's post-diff impact analyzer
  remains the safety net" / "the runner will analyze the final diff". On Graph that is false.

Expected: a Graph run of a high-impact task warns before dispatch when the declared scope looks
incomplete, and fails the attempt with a clear message when a reverse-dependent crate no longer compiles.

## Why it matters

Goal `core`: public-contract edits are where agent attempts burn retries and need hand repair. The prompt
currently promises a check that does not happen, so agents may stop early trusting it. Related:
`gap-a247a4` (owns the `roko-index` semantic consumer oracle, the second half of this spec's title),
`gap-870e25` (parked, same residual), `gap-1426e4` (adaptive verify scoping, touches the same function),
`gap-4a6dcb` (FAST on Graph; Runner-v2 ran impact analysis only in focused gate mode).

## Where

- `crates/roko-cli/src/runner/impact_analysis.rs` (1991 lines): `analyze(workdir, planned_files,
  gates_config)`, `analyze_against(..)`, `ImpactReport { producer_packages, reverse_dependents, targets,
  high_impact, high_impact_reasons, confidence, analysis_ms, fallback_reason, .. }`, `classify_diff`,
  `try_symbol_oracle` (labels only). Unit fixtures already exist here:
  `public_struct_field_change_with_mock_workspace`, `transitive_reverse_dependents_via_mock`,
  `private_helper_body_edit_no_cross_crate_signal`, `reexport_and_serde_consumer_detected`,
  `reverse_dependents_overflow_cap`.
- Live callers: `commands/impact.rs::cmd_impact` (`roko impact`), and
  `graph_execution/plan_set.rs` (`cargo_metadata`, `reverse_dependents` for plan footprints).
- Dead on Graph: `runner/gate_dispatch.rs::run_gate_once` (about lines 917-1000: impact analysis with a
  timeout in `GateMode::Focused`, then `runner/cargo_command.rs::focused_verify_steps` (line 709) and
  `with_targeted_compile_rung` (line 588)); `runner/preflight.rs::check_declared_change_impact` (line
  132: warns when a task declares a likely public change without `context.symbols`, or lists one file;
  honours `impact_acknowledgement`).
- Graph verify: `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification`
  (about line 1807) runs only the authored `[[task.verify]]` steps. Entry: `roko plan run` ->
  `graph_execution/plan_runner.rs::run_graph_plan` -> `GraphTaskDispatcher::dispatch`.
- Prompt: `crates/roko-cli/src/dispatch/prompt_builder.rs::declared_impact_context` (line 277): keyword
  heuristic (`public`, `signature`, `trait`, `serde`, ...), not the `ImpactReport`.
- Task schema: `task_parser.rs` `context.symbols`, `impact_acknowledgement` (line 750).

## Current state

Done and live: the analysis library and its unit fixtures (spec checklist 1-3 at analysis level), the
`roko impact` CLI, the prompt impact policy, the author escape hatch field. Not live on Graph: the
preflight diagnostic, the reverse-dependent compile gate, planned-vs-actual telemetry from the gate.
Not started: the runtime fixture (spec acceptance 1), plan-generation `read_files` enrichment, latency
reporting, fixed-SHA scoped-vs-full benchmark. The August status notes below predate the deletion.

## Plan

1. Pre-dispatch warning: make `check_declared_change_impact` `pub(crate)` and call it from
   `run_graph_plan` after plans load (warn, never block), and from `roko plan validate`.
2. Post-diff gate in `settle_task_verification`, after the authored steps pass: run
   `impact_analysis::analyze(&effective_workdir, &task.files, &self.config.gates)` under
   `tokio::time::timeout(gates.impact_timeout_ms)`. If `report.high_impact` and there are
   `reverse_dependents`, run `cargo check` for them (reuse `focused_verify_steps` / the `targets`'
   `check_command()`, capped by `gates.impact_max_reverse_dependents` and `impact_max_targets`). A
   failure is a `RokoError::Verify` naming the crate, so the Graph retry sees it. A timeout or low
   confidence widens to `cargo check --workspace` rather than skipping. Skip when the task has
   `impact_acknowledgement`.
3. Design choice: (a) enforce as above (recommended: this is the dogfood failure, and the cost is only
   paid on high-impact diffs); (b) record-only telemetry; (c) do not wire, and delete the "safety net"
   sentences from `declared_impact_context`. If (a) is not done now, do (c) at least, so the prompt
   stops promising a check that never runs.
4. Remove the three `gates.impact_*` entries from `graph_engine_inert_settings` once they are read.
5. Record `report.analysis_ms`, targets and misses (actual changed files outside `task.files`) in the
   gate record or efficiency event, separate from agent time.
6. Runtime fixture `graph_impact_gate_compiles_reverse_dependents`: a temp Cargo workspace with crate `a`
   (`pub struct S { pub flag: bool }`) and crates `b`, `c` that read `flag`; a mock agent changes only
   `a` to `Option<bool>`; the Graph attempt fails at the impact gate naming `b` and `c`. A second case
   with a private helper edit passes without cross-crate checks.
7. Leave the `roko-index` oracle to `gap-a247a4`, and the fixed-SHA benchmark out of this item.

## Done when

- `roko plan run` warns before dispatch for a high-impact task without `context.symbols`.
- A Graph attempt that breaks a reverse-dependent crate fails with a message naming it; a private edit
  does not trigger cross-crate checks.
- The prompt's impact text matches what the runner does.
- Verify: `grep -rqw 'fn graph_impact_gate_compiles_reverse_dependents' crates/roko-cli/ && cargo test -p roko-cli graph_impact_gate_compiles_reverse_dependents && ! grep -A1 -E '"gates\.impact_(timeout_ms|max_reverse_dependents|max_targets)"' crates/roko-cli/src/graph_task_dispatch.rs | grep -q LEGACY_GATES`

## Notes

- The fixture compiles real crates; keep it tiny, use a temp target dir, and expect it to take tens of
  seconds. Mark it `#[cfg(unix)]` if it uses the mock-agent `PATH` shim from `tests/graph_plan_callers.rs`.
- Extra `cargo check` runs contend for the build lock with parallel plans (`[conductor]
  max_parallel_plans`); take a permit the way authored cargo steps do (`verify_compile_permit` ->
  `runner::gate_dispatch::acquire_compile_ownership`, `graph_task_dispatch.rs` about line 3968).
- `settle_task_verification` is also the anchor of `gap-1426e4` and of the FAST work in `gap-4a6dcb`;
  coordinate or sequence those edits.
- Imported spec: status notes below are from 2026-09-01, before Runner-v2 was deleted.

## Original notes


Imported 2026-09-29 from `tmp/backlog/231-cross-crate-change-impact-scoping.md` (tmp/backlog is frozen). Not yet checked against current code: the text below is the spec as last written, including its own status notes.

## Original spec

# 231 — Cross-Crate Change-Impact Scoping Before Dispatch

> **Status: PARTIAL — CONSERVATIVE IMPACT/GATE PATH IMPLEMENTED** (2026-08-31,
> `d43dd45cd`). Actual Git diffs and Cargo metadata now select exact targets/features, widen shared
> modules, identify likely public/re-export/trait/serde contracts, compile bounded transitive
> reverse dependents, emit scope diagnostics, and record planned-vs-actual misses. A complete
> symbol-level `roko-index` consumer oracle for macro-generated APIs and non-Rust schemas remains
> open; ambiguity safely widens/escalates. The fixed-SHA scorecard runner (`d1b94b139`) can now
> measure scoped versus full lanes, but the real latency and escaped-regression fixtures are still
> pending. The final checkpoint built the integrated CLI and passed its complete library harness;
> this proves the implemented path compiles/tests at library scope, not the still-open public-
> contract impact fixtures below.

**Priority**: P1 — incomplete file scope caused expensive retry loops and manual repair after public type changes
**Size**: M (2–3 days)
**Wave**: 5
**Crates**: `roko-cli`, `roko-index`
**Depends on**: #195 (file overlap analysis, done), #205 (`get_plan_context` MCP tool, soft), #210 (code-index prompt section, soft)
**Source**: `tmp/archive/dogfood-2026-08-25/DOGFOOD-DEBRIEF.md`

## Background

The efficiency `gate_passed` dogfood task changed a public type from `bool` to `Option<bool>`.
The agent correctly changed the producer crate and some CLI consumers, but the authored task listed
only five files and omitted more than ten downstream call sites across `roko-serve` and `roko-cli`.
Two agent attempts failed compile gates and the remaining consumers were repaired manually.

Current plan generation can add `context.read_files`, file-overlap analysis can detect collisions
between planned tasks, and gates can target crates. None of those surfaces asks the inverse
question before dispatch: “If these public symbols/signatures change, which workspace crates and
call sites must also change and which reverse dependents must compile?”

## Implementation Plan

- [x] Add a conservative impact-analysis pass during plan validation/preflight. Starting from each task's declared
   files and named symbols, use syntax signals plus `cargo metadata` to identify public definitions,
   re-exports, trait/serde signals, target ownership, and reverse-dependent crates. Exact semantic
   call-site enumeration remains open.
- [x] Classify risk. Signature/type/enum/trait/serialized-schema changes are high-impact; private
   function-body edits are low-impact. Emit stable diagnostics when high-impact tasks omit known
   consumers or restrict gates to only the producer crate.
- [x] Provide an explicit plan author escape hatch with a reason (for false positives or staged
   migrations), but do not silently expand the writable file allowlist at execution time.
- [x] Feed the impact report into plan-generation/preflight prompts and the implementation agent's context.
   Require the agent to search all call sites before editing a public surface and to report any
   newly discovered consumers.
- [x] Expand compile verification to reverse-dependent crates for high-impact changes, bounded by a
   configurable cap. Reuse the shared target directory so correctness does not reintroduce cold
   build cost.
- [x] Persist the planned-vs-actual file set and impact decisions as structured telemetry.

## Acceptance Criteria

- [ ] A final fixture changing a public struct field reports/compiles at least two reverse-dependent
   crates before agent dispatch.
- [x] High-impact plans with a likely incomplete `files` list receive an actionable validation diagnostic.
- [x] Private implementation-only edits remain target-scoped rather than automatically workspace-wide.
- [x] Reverse-dependent compile gates are source-implemented with one shared compile owner and the
  integrated CLI/library checkpoint is green; the dedicated runtime impact fixture is pending.
- [x] The agent prompt contains the impact report and planned-vs-actual files are recorded.
- [x] False positives can be acknowledged explicitly without disabling the analysis globally.

## Verification Checklist

- [ ] Fixture: `bool` → `Option<bool>` public field with consumers in three crates; all consumers are
      reported and reverse-dependent compilation is selected.
- [ ] Fixture: private helper body edit; no cross-crate warning or expanded gate.
- [ ] Fixture: re-exported enum variant and serde snapshot consumer; both are found.
- [ ] Generate a plan from a backlog spec containing a public API change; verify `context.read_files`
      includes representative consumers.
- [ ] Confirm analysis latency remains bounded and is reported separately from agent time.
- [ ] Run fixed-SHA cold/warm repetitions and compare escaped regressions against the full-CI
      baseline; benchmark tooling alone is not proof.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/plan.rs` | Invoke impact analysis for generate/validate/run preflight |
| `crates/roko-cli/src/runner/preflight.rs` | Emit scope diagnostics before dispatch |
| `crates/roko-cli/src/runner/gate_dispatch.rs` | Compile affected reverse dependents |
| `crates/roko-cli/src/runner/impact_analysis.rs` | Diff, Cargo target/feature, public-signal, and reverse-dependency analysis |
| `crates/roko-cli/src/dispatch/prompt_builder.rs` | Include bounded impact results in agent context |
| `crates/roko-core/src/config/gates.rs` | Explicit gate breadth and impact caps |

The originally proposed `roko-index` symbol/call-site query remains the residual needed to upgrade
this item from conservative partial coverage to complete semantic impact analysis.

## Status Update (2026-09-01)

**Overall: PARTIAL -- conservative impact path implemented; runtime fixtures open.** The core
implementation landed in `d43dd45cd` ("feat(gates): add impact-aware fast verification") with
a subsequent hardening pass in `38f79a5ca` ("fix: finalize development speed integration").
No new code has landed since 2026-08-31.

**Verification fixtures (2026-09-01):** Verification fixtures added to
`crates/roko-cli/src/runner/impact_analysis.rs`: public struct field change reports reverse
dependents, private helper body edit produces no cross-crate signal, re-exported enum variant
and serde consumer are both detected. The `classify_diff_lines` helper has been extracted for
testability. Full cross-crate transitivity and runtime impact fixtures remain open.

### Verified current state

`crates/roko-cli/src/runner/impact_analysis.rs` (907 lines) implements:
- Git diff parsing to identify changed files per task.
- `cargo_metadata` consumption for target ownership, features, and reverse-dependent crates.
- Public/re-export/trait/serde signal identification from syntax patterns.
- Bounded transitive reverse-dependent compilation selection.
- Stable diagnostic emission for high-impact tasks with incomplete file lists.
- Planned-vs-actual file set persistence as structured telemetry.
- Explicit author escape hatch for false positives.

The module exports an `analyze` async function and supporting types (`ImpactAnalysis`,
`ImpactedTarget`). The gate_dispatch module in the runner consumes it for compile scope
widening.

### What remains open

All six verification checklist items are unchecked. These are runtime fixture exercises:

1. `bool` to `Option<bool>` public field fixture with three-crate consumers.
2. Private helper body edit (no cross-crate expansion).
3. Re-exported enum variant + serde snapshot consumer fixture.
4. Plan generation with public API change and consumer read_files.
5. Analysis latency measurement and separation from agent time.
6. Fixed-SHA cold/warm repetitions comparing scoped vs full CI.

The first acceptance criterion (final fixture proving reverse-dependent compile before
dispatch) is also unchecked.

Additionally, the `roko-index` symbol/call-site query oracle for macro-generated APIs and
non-Rust schemas is explicitly deferred. The current implementation widens/escalates on
ambiguity rather than querying exact call sites.

### Audit cross-references

- **cli-audit**: No direct mention of impact analysis or cross-crate scoping. The audit
  focused on CLI command surfaces, not runner preflight internals.
- **engine-audit `05-duplicate-paths.md`**: Notes cross-crate duplication of model resolution
  and config loading functions (11 and 9 copies respectively). These are exactly the kind of
  widely-consumed internal APIs where a signature change would trigger #231's impact analysis.
  Consolidating them (as the engine roadmap proposes) would reduce the blast radius that
  #231 must detect.
- **engine-audit `12-do-cmd-routing.md`**: References intent classification routing that
  touches cross-crate boundaries; impact analysis should cover these paths when they change.
- **engine-audit `IMPLEMENTATION-ROADMAP.md`**: The convergence roadmap's #243 (unified
  RuntimeServices) and #258-#278 (Cell extraction) will themselves be high-impact cross-crate
  changes. #231's conservative impact analysis should be exercised as a preflight check during
  those refactors. The roadmap does not explicitly reference #231 but its integration gates
  require "affected packages/fixtures identified" evidence, which is what #231 produces.
- **ux-audit**: Empty (no files).

### Recommendation

The conservative implementation is sound and covers the original dogfood failure scenario.
The open items are all fixture/proof work. The `roko-index` oracle upgrade should be deferred
until after the engine-audit convergence work stabilizes the public API surface, since that
work will both exercise and potentially reshape the impact-analysis boundaries.
