# LERN_04: Wire FeedbackService to ACP Pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-04`](../ISSUE-TRACKER.md#lern-04)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.4
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The ACP runner (`roko-acp/src/runner.rs`) handles editor integration (VS Code, etc.). At line 1666-1667, it defines `THRESHOLDS_PATH` and writes adaptive gate thresholds. It does not import or use `FeedbackService`. The pipeline (`pipeline.rs`) orchestrates ACP model calls and gate runs. Neither emits `FeedbackEvent`s.

`roko-acp/Cargo.toml` needs `roko-learn` as a dependency (check if already present; `roko-learn` may already be pulled transitively through `roko-core`).

## Exact Changes

1. Add `roko-learn` to `roko-acp/Cargo.toml` `[dependencies]` if not present.
2. In ACP runner initialization, create `FeedbackService::from_roko_dir_with_episodes(&workdir.join(".roko"))`.
3. Store the service on the runner struct or pass through the pipeline.
4. After each ACP model dispatch in `pipeline.rs`, emit `FeedbackEvent::ModelCall` with `role: "acp"`.
5. After each ACP gate run in `runner.rs` (where it currently writes only adaptive thresholds), emit `FeedbackEvent::GateResult` alongside the existing threshold write.
6. Flush on pipeline completion.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-acp/src/pipeline.rs`
- `crates/roko-acp/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP pipeline emits ModelCall events visible in `.roko/learn/efficiency.jsonl`
- [ ] Gate threshold writes still work unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP pipeline emits ModelCall events visible in `.roko/learn/efficiency.jsonl`
- Gate threshold writes still work unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
