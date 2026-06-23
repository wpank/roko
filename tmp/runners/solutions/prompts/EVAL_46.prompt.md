# EVAL_46: Wire EvalService into the runtime

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-46`](../ISSUE-TRACKER.md#eval-46)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.46
- Priority: **P0**
- Effort: 10 hours
- Depends on: `EVAL_04` (source 5.4), `EVAL_05` (source 5.5), `EVAL_06` (source 5.6), `EVAL_08` (source 5.8), `EVAL_09` (source 5.9), `EVAL_10` (source 5.10), `EVAL_11` (source 5.11), `EVAL_40` (source 5.40)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_46 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After agent dispatch produces output, the runtime must construct `ArtifactRef` from task workdir, select evaluation profile, run `EvalService::evaluate()`, project `EvalTrace` to `GateVerdict`, and emit both `RuntimeEvent::EvalCompleted` and gate events.

The profile selection logic: check `task.eval_profile` -> `plan.eval_profile` -> `roko.toml [eval.default_profile]` -> built-in `rust-strict` for Rust projects.

## Exact Changes

1. Construct `BridgeGateService` wrapping `GateService` at startup.
2. Migrate the compile, lint, test, format, and diff gates (add their names to `BridgeGateService.migrated`).
3. After agent dispatch, construct `ArtifactRef` from task workdir.
4. Select profile via the cascade: task config -> plan config -> workspace config -> default.
5. Run `EvalService::evaluate()` through the bridge.
6. Feed eval outcomes to:
   - `EpisodeLogger` (eval_trace_id in episode.extra)
   - `FeedbackService` (via feedback_bridge output)
   - `ExperimentStore` (via prompt_variant)
   - `AdaptiveThresholds` (via per-criterion stats observation)
   - `PreferenceTriple` logger
7. Emit `RuntimeEvent::EvalCompleted`.

## Design Guidance

This is the highest-risk task. The bridge pattern ensures zero regression: if `EvalService` fails, fall back to legacy `GateService`. Log warnings when fallback occurs. The existing gate call site must be found in the orchestrator -- search for `run_gates` or `GateRunner` calls.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`

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

- [ ] End-to-end: `roko plan run` with a simple task produces an `EvalTrace` in `.roko/eval/traces.jsonl`
- [ ] Events appear on the runtime event bus
- [ ] Legacy gate behavior is preserved for non-migrated gates

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_46 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- End-to-end: `roko plan run` with a simple task produces an `EvalTrace` in `.roko/eval/traces.jsonl`
- Events appear on the runtime event bus
- Legacy gate behavior is preserved for non-migrated gates
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_46 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
