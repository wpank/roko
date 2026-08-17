# INNO_55: Add vendor-neutral observability config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-55`](../ISSUE-TRACKER.md#inno-55)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.55
- Priority: **P3**
- Effort: 4 hours
- Depends on: `INNO_54` (source 11.54)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_55 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Add `[observability]` section to roko.toml schema:
   `provider`, `endpoint`, `protocol`, `api_key_env`.
2. Configure OTLP exporter based on provider.
3. Validate config at startup.

## Write Scope

- `crates/roko-core/src/config/schema.rs`
- `crates/roko-runtime/src/otel_emitter.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Changing `provider` from `langfuse` to `honeycomb` requires only config change
- [ ] Missing API key produces a clear error at startup
- [ ] `roko config validate` checks observability config

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_55 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Changing `provider` from `langfuse` to `honeycomb` requires only config change
- Missing API key produces a clear error at startup
- `roko config validate` checks observability config
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_55 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
