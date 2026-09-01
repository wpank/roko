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
