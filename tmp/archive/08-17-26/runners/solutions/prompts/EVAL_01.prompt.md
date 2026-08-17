# EVAL_01: Scaffold `roko-eval` crate with core types

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-01`](../ISSUE-TRACKER.md#eval-01)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.1
- Priority: **P0**
- Effort: 6 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

No `roko-eval` crate exists yet. The crate will house the evaluation framework kernel.
Dependencies: `roko-core`, `async-trait`, `serde`, `serde_json`, `thiserror`, `uuid`, `chrono`.
Register in workspace `Cargo.toml` under `members` (NOT `default-members` -- it is a library, not a shipped binary).

## Exact Changes

1. Create `crates/roko-eval/Cargo.toml` with dependencies on `roko-core = { path = "../roko-core" }`, `async-trait`, `serde`, `serde_json`, `thiserror`, `uuid`, `chrono`.
2. Create `crates/roko-eval/src/lib.rs` declaring the crate's modules and re-exports.
3. Create `crates/roko-eval/src/types.rs` defining the core types:
   - `EvidenceKind` enum (ProcessOutput, ProcessStatus, Diff, SemanticDiff, Ast, RuntimeTrace, StaticAnalysis, Custom). Start with the evidence kinds that existing gates produce. New kinds (Dom, ComputedStyles, Screenshot, etc.) are added when criteria need them.
   - `ArtifactRef` struct (id: String, path: Option<PathBuf>, url: Option<String>, artifact_type: String, content_hash: Option<ContentHash>, metadata: serde_json::Value).
   - `EvidenceBag` struct wrapping `Vec<EvidenceItem>` with typed accessors: `get_one(kind) -> Option<&EvidenceItem>`, `get_all(kind) -> Vec<&EvidenceItem>`, `has(kind) -> bool`, `insert(item)`.
   - `EvidenceItem` struct (kind: EvidenceKind, data: serde_json::Value, source: String, collected_at: DateTime<Utc>, content_hash: Option<ContentHash>).
   - `Severity` enum (Critical, Hard, Soft, Info).
   - `CriterionKind` enum (Deterministic, Computed, Heuristic, JudgePanel, Script).
   - `Finding` struct (criterion: String, severity: Severity, summary: String, detail: Option<String>, source_location: Option<SourceLocation>, rule_id: Option<String>, source_tool: Option<String>, fix_hint: Option<String>, confidence: Option<f64>).
   - `SourceLocation` struct (file: String, line: Option<u32>, col: Option<u32>, end_line: Option<u32>, end_col: Option<u32>).
   - `CriterionResult` struct (criterion_name: String, kind: CriterionKind, score: f64, passed: bool, findings: Vec<Finding>, metadata: Option<serde_json::Value>, duration_ms: u64, cost_usd: f64).
   - `EvalVerdict` struct (passed: bool, score: f64, hard_failures: Vec<String>, soft_scores: Vec<(String, f64)>, findings: Vec<Finding>, criteria_passed: usize, criteria_total: usize).
   - `EvalError` enum (EvidenceUnavailable { kind: EvidenceKind, collector: String }, Evaluation(String), CollectorFailed { name: String, source: String }, Timeout { duration_ms: u64 }, Configuration(String), Internal(String)).
4. Add `"crates/roko-eval"` to workspace `members` in `/Users/will/dev/nunchi/roko/roko/Cargo.toml`.

## Design Guidance

All types must derive `Debug, Clone, Serialize, Deserialize`. Use `#[serde(rename_all = "snake_case")]` for enums. `EvidenceBag` should be `#[derive(Default)]` so empty bags can be constructed trivially. The `EvalError` should implement `std::error::Error` via `thiserror`.

## Write Scope

- `Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All types round-trip through serde_json

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All types round-trip through serde_json
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
