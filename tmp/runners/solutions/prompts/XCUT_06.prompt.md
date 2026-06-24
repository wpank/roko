# XCUT_06: Unify Logging Initialization Across Entry Points

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-06`](../ISSUE-TRACKER.md#xcut-06)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.6
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko serve`, `roko plan run`, `roko chat`, and `roko agent serve` each initialize logging differently. The CLI main.rs sets up `tracing_subscriber` with `EnvFilter`; the ACP server has its own log file configuration in `crates/roko-acp/src/config.rs`. The daemon has yet another initialization path.

## Exact Changes

1. Create `roko_runtime::logging::init(config: &LogConfig)` that handles all logging setup.
2. `LogConfig` reads from `roko.toml` `[logging]` section: `level`, `format` (text/json), `file` (optional path), `otel_endpoint` (optional).
3. The init function sets up: `tracing_subscriber::fmt` layer + optional JSONL file layer + optional OTel layer.
4. Replace all ad-hoc logging init in `main.rs`, `daemon.rs`, and ACP `config.rs` with the single `init()` call.
5. Ensure `RUST_LOG` env var overrides take precedence over config.

## Write Scope

- `crates/roko-runtime/src/lib.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/daemon.rs`
- `crates/roko-acp/src/config.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All CLI entry points use `roko_runtime::logging::init()`
- [ ] `roko serve` and `roko plan run` produce identically structured log output at the same level
- [ ] `[logging] format = "json"` in roko.toml produces structured JSON log lines

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All CLI entry points use `roko_runtime::logging::init()`
- `roko serve` and `roko plan run` produce identically structured log output at the same level
- `[logging] format = "json"` in roko.toml produces structured JSON log lines
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
