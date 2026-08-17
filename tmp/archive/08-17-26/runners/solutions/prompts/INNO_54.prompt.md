# INNO_54: Implement OTel gen_ai.* span emission

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-54`](../ISSUE-TRACKER.md#inno-54)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.54
- Priority: **P3**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_54 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

OTel gen_ai.* semantic conventions (v1.37+) define standard attributes for LLM
observability. Langfuse, Phoenix, Honeycomb all support OTLP ingestion.

## Exact Changes

1. Add `opentelemetry`, `opentelemetry-otlp`, `opentelemetry-sdk` to
   roko-runtime dependencies (feature-gated under `otel`).
2. Define `OtelEmitter` struct wrapping a tracer provider.
3. On each agent dispatch, create a span with gen_ai.* attributes.
4. On each gate evaluation, create a child span.
5. Emit to configurable OTLP endpoint (from roko.toml).
6. Feature-gate: only active when `[observability]` config is present.

## Write Scope

- `crates/roko-runtime/src/lib.rs`
- `crates/roko-runtime/Cargo.toml`

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

- [ ] With `[observability] provider = "otlp-generic"` configured, OTel spans are emitted
- [ ] Spans include all gen_ai.* attributes per v1.37+ spec
- [ ] Without observability config, no OTel overhead (feature-gated)
- [ ] JSONL logging continues alongside OTel (not replaced)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_54 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `[observability] provider = "otlp-generic"` configured, OTel spans are emitted
- Spans include all gen_ai.* attributes per v1.37+ spec
- Without observability config, no OTel overhead (feature-gated)
- JSONL logging continues alongside OTel (not replaced)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_54 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
